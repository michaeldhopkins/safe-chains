use crate::parse::{Token, WordSet};
use crate::registry;
use crate::verdict::{SafetyLevel, Verdict};

// PHP_SAFE_INI_DIRECTIVES is data, but it is tightly coupled to the
// `-d KEY=VALUE` pre-flag-scan validation (see is_safe_ini_pair) which
// the declarative TOML schema doesn't express — the value must be a
// `KEY=VALUE` pair where the KEY is in this allowlist. Adding a TOML
// primitive for "valued flag with key allowlist for KEY=VALUE form" is
// a future refinement; until then this list lives in Rust as a known
// exception to the data-in-TOML principle.
static PHP_SAFE_INI_DIRECTIVES: WordSet = WordSet::new(&[
    "date.timezone", "display_errors", "error_reporting", "max_execution_time", "max_input_time", "max_input_vars", "memory_limit",
    "opcache.enable", "opcache.enable_cli", "post_max_size", "upload_max_filesize",
]);

fn is_safe_ini_pair(value: &str) -> bool {
    let Some((key, _)) = value.split_once('=') else {
        return false;
    };
    PHP_SAFE_INI_DIRECTIVES.contains(key)
}

pub fn is_safe_php(tokens: &[Token]) -> Verdict {
    let mut i = 1;
    while i < tokens.len() {
        let s = tokens[i].as_str();
        if s == "-d" {
            let Some(next) = tokens.get(i + 1) else {
                return Verdict::Denied;
            };
            if !is_safe_ini_pair(next.as_str()) {
                return Verdict::Denied;
            }
            i += 2;
            continue;
        }
        if let Some(rest) = s.strip_prefix("-d") {
            if !is_safe_ini_pair(rest) {
                return Verdict::Denied;
            }
            i += 1;
            continue;
        }
        break;
    }

    let Some(arg) = tokens.get(i) else {
        return Verdict::Denied;
    };
    let arg_str = arg.as_str();

    // `-l` / `--syntax-check` PARSES each operand and reports syntax errors. It does NOT run the
    // file — verified against php 8: a script whose body echoes prints only "No syntax errors
    // detected". So this is a READ of each operand, gated by locus like any other reader, and not
    // the code-execution surface a bare script path is.
    //
    // Found by the decision log: `php -l <file>` was denied while `ruby -c` and `node --check` —
    // the same operation in two other languages — were both allowed. The TOML fallback covers the
    // no-operand diagnostics (`--version`, `-i`, `-m`) at `max_positional = 0`, so a lint with a
    // file could not be expressed there.
    //
    // At least one operand required. Bare `php -l` lints STDIN, which is harmless but is also a
    // form nobody types deliberately; refusing it keeps this narrow.
    if matches!(arg_str, "-l" | "--syntax-check") {
        let files = &tokens[i + 1..];
        if files.is_empty() {
            return Verdict::Denied;
        }
        // Every remaining token must be a plain FILE. Treating them all as path operands and
        // gating them as reads was a fail-open: `php -l -S localhost:8080` gated `-S` and
        // `localhost:8080` as two worktree-relative reads and approved — while php really does
        // start the built-in HTTP server, verified listening on the port. `-S` is out of coverage
        // and has its own denied! case; the lint branch was walking around it.
        //
        // Rejecting every dash-led token rather than the known-dangerous ones, because that is the
        // allowlist-shaped rule: `-l` takes files. It costs `php -l -f x.php`, which does lint
        // (checked) but is a form nobody needs when `php -l x.php` is right there.
        if files.iter().any(|f| f.as_str().starts_with('-')) {
            return Verdict::Denied;
        }
        let mut level = SafetyLevel::Inert;
        for f in files {
            match crate::engine::resolve::read_content_verdict(f.as_str()) {
                Verdict::Denied => return Verdict::Denied,
                Verdict::Allowed(l) => level = level.max(l),
            }
        }
        return Verdict::Allowed(level);
    }

    // Bare diagnostic flag as the final token: probe the [command.fallback]
    // grammar with a synthetic two-token slice so `php -d X=1 --version`
    // is validated the same way as `php --version`.
    if i + 1 == tokens.len() {
        let probe = [tokens[0].clone(), arg.clone()];
        if let Some(v @ Verdict::Allowed(_)) = registry::try_fallback_grammar("php", &probe) {
            return v;
        }
    }

    // Delegate-by-basename: the script path's basename must match a
    // declared sub (`artisan`, `please`). delegate_skip = 0 on the sub
    // dispatches the (normalized) token tail to command_verdict so the
    // inner top-level command's TOML drives validation.
    let basename = std::path::Path::new(arg_str).file_name().and_then(|s| s.to_str()).unwrap_or(arg_str);
    let mut probe = Vec::with_capacity(tokens.len() - i + 1);
    probe.push(tokens[0].clone());
    probe.push(Token::from_raw(basename.to_string()));
    for t in &tokens[i + 1..] {
        probe.push(t.clone());
    }
    registry::try_sub_dispatch("php", &probe).unwrap_or(Verdict::Denied)
}

// Laravel's cache:clear is treated as SafeWrite for any well-formed
// invocation, including bare (which uses config('cache.default')) and
// any explicit store value.
//
// SafeWrite is normally local-only, and cache:clear with a redis,
// memcached, database, or dynamodb store does interact with a remote
// system. We deviate here on a recoverability argument: cache stores
// are by definition recreatable, so a flush — even on prod — is a
// transient performance event, not data loss. Worst-case downstream
// effects (sessions cleared if SESSION_DRIVER is cache-backed; rate-
// limit counters reset; Cache::lock() mutex keys cleared; warmed
// `Cache::remember()` entries gone) are all self-healing within
// seconds to minutes of normal traffic.
//
// For local cache:clear to actually reach prod, the local Laravel
// installation must be configured with prod's cache-backend
// connection details (REDIS_HOST=prod-redis, etc.) — an established
// anti-pattern in Laravel dev that the tool can't (and shouldn't)
// detect from CLI tokens alone. We trust the user's environment
// configuration, the same way every other Artisan command does.
//
// The handler still validates the *shape* of the invocation (only
// known flags / one positional store / one positional --tags value)
// to avoid passing through arbitrary token streams.
pub fn check_laravel_cache_clear(tokens: &[Token]) -> Verdict {
    if tokens.len() == 2 && matches!(tokens[1].as_str(), "--help" | "-h") {
        return Verdict::Allowed(SafetyLevel::Inert);
    }

    let mut saw_positional = false;
    let mut i = 1;
    while i < tokens.len() {
        let s = tokens[i].as_str();
        if s == "--help" || s == "-h" {
            i += 1;
            continue;
        }
        if s == "--store" || s == "--tags" {
            if tokens.get(i + 1).is_none() {
                return Verdict::Denied;
            }
            i += 2;
            continue;
        }
        if s.starts_with("--store=") || s.starts_with("--tags=") {
            i += 1;
            continue;
        }
        if !s.starts_with('-') && !saw_positional {
            saw_positional = true;
            i += 1;
            continue;
        }
        return Verdict::Denied;
    }

    Verdict::Allowed(SafetyLevel::SafeWrite)
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    use crate::verdict::{SafetyLevel, Verdict};

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    fn verdict(cmd: &str) -> Verdict {
        crate::command_verdict(cmd)
    }

    safe! {
        php_help: "php --help",
        php_help_short: "php -h",
        php_version: "php --version",
        php_version_short: "php -V",
        php_modules: "php -m",
        php_info: "php -i",
        php_ini: "php --ini",
        php_d_memory_then_artisan: "php -d memory_limit=512M artisan view:clear",
        php_d_attached_then_artisan: "php -dmemory_limit=512M artisan view:clear",
        php_d_then_please: "php -d memory_limit=512M please stache:clear",
        php_d_then_path_please: "php -d memory_limit=512M /Users/me/projects/site/please stache:clear",
        php_path_artisan: "php /var/www/app/artisan view:clear",
        php_path_please: "php /Users/me/projects/blog/please stache:clear",
        php_two_d_flags: "php -d memory_limit=512M -d max_execution_time=300 artisan view:clear",
        php_d_diagnostic_then_version: "php -d display_errors=1 --version",
        php_d_timezone: "php -d date.timezone=UTC artisan view:clear",
        php_artisan_cache_clear_file: "php artisan cache:clear --store=file",
        php_artisan_cache_clear_array: "php artisan cache:clear --store=array",
        php_artisan_cache_clear_null: "php artisan cache:clear --store=null",
        php_artisan_cache_clear_file_space: "php artisan cache:clear --store file",
        php_please_cache_clear_file: "php please cache:clear --store=file",
        php_path_please_cache_clear: "php /path/please cache:clear --store=file",
        php_d_then_cache_clear: "php -d memory_limit=512M /path/please cache:clear --store=file",
        php_artisan_cache_clear_help: "php artisan cache:clear --help",
        php_artisan_cache_clear_positional_file: "php artisan cache:clear file",
        php_artisan_cache_clear_positional_array: "php artisan cache:clear array",
        php_artisan_cache_clear_positional_null: "php artisan cache:clear null",
        php_please_cache_clear_positional_file: "php please cache:clear file",
        php_path_please_cache_clear_positional: "php /path/please cache:clear file",
        php_d_then_cache_clear_positional: "php -d memory_limit=512M /path/please cache:clear file",
        php_artisan_cache_clear_bare: "php artisan cache:clear",
        php_please_cache_clear_bare: "php please cache:clear",
        php_artisan_cache_clear_redis: "php artisan cache:clear --store=redis",
        php_artisan_cache_clear_database: "php artisan cache:clear --store=database",
        php_artisan_cache_clear_memcached: "php artisan cache:clear --store=memcached",
        php_artisan_cache_clear_dynamodb: "php artisan cache:clear --store=dynamodb",
        php_artisan_cache_clear_positional_redis: "php artisan cache:clear redis",
        php_artisan_cache_clear_positional_glide: "php artisan cache:clear glide",
        php_artisan_cache_clear_positional_database: "php artisan cache:clear database",
        php_artisan_cache_clear_with_tags: "php artisan cache:clear --tags=foo --store=file",
        php_artisan_cache_clear_tags_only: "php artisan cache:clear --tags=foo",
        php_artisan_cache_clear_tags_space: "php artisan cache:clear --tags foo",
        // `-l` parses and reports syntax errors WITHOUT running the file (verified on php 8: a
        // script whose body echoes prints only "No syntax errors detected"). It is the same
        // operation as `ruby -c` and `node --check`, both of which were already allowed — the
        // decision log caught php being the odd one out.
        php_lint: "php -l pages/team.php",
        php_lint_long: "php --syntax-check pages/team.php",
        php_lint_after_ini: "php -d memory_limit=512M -l pages/team.php",
        php_lint_several: "php -l a.php b.php",
    }

    denied! {
        php_bare: "php",
        php_run_code: "php -r 'echo 1;'",
        php_script_random: "php /tmp/random.php",
        php_basename_not_allowed: "php /tmp/script.php arg",
        php_d_unsafe_directive: "php -d auto_prepend_file=/etc/passwd artisan view:clear",
        php_d_disable_functions: "php -d disable_functions= artisan view:clear",
        php_d_open_basedir: "php -d open_basedir=/ artisan view:clear",
        php_d_include_path: "php -d include_path=. artisan view:clear",
        php_d_no_equals: "php -d memory_limit artisan view:clear",
        php_d_attached_unsafe: "php -dauto_prepend_file=/tmp/x artisan view:clear",
        php_d_consumes_artisan: "php -d artisan view:clear",
        // A lint still READS its operand, so the operand is gated like any other read.
        // And a FLAG after `-l` is not a file: `php -l -S localhost:8080` really does start the
        // built-in server (verified listening on the port) — the first cut of the lint branch
        // gated `-S` and the host:port as two worktree reads and approved it.
        php_lint_then_server: "php -l -S localhost:8080",
        php_lint_then_run_code: "php -l -r 'system(id);'",
        php_lint_then_f: "php -l -f evil.php",
        php_lint_shielded_file: "php -l ~/.ssh/id_rsa",
        php_lint_system_secret: "php -l /etc/shadow",
        php_lint_no_operand: "php -l",
        php_built_in_server: "php -S localhost:8000",
        php_built_in_server_with_t: "php -S localhost:8000 -t public",
        php_help_then_extra: "php --help artisan",
        php_unrecognized_flag: "php --unknown artisan view:clear",
        php_artisan_cache_clear_unknown_flag: "php artisan cache:clear --store=file --foo",
        php_artisan_cache_clear_store_no_value: "php artisan cache:clear --store",
        php_artisan_cache_clear_tags_no_value: "php artisan cache:clear --tags",
        php_path_basename_only: "php /tmp/please-bak stache:clear",
        php_artisan_cache_clear_two_positionals: "php artisan cache:clear file array",
    }

    #[test]
    fn cache_clear_positional_file_is_safewrite() {
        assert_eq!(verdict("php artisan cache:clear file"), Verdict::Allowed(SafetyLevel::SafeWrite));
    }

    #[test]
    fn cache_clear_bare_is_safewrite() {
        assert_eq!(verdict("php artisan cache:clear"), Verdict::Allowed(SafetyLevel::SafeWrite));
    }

    #[test]
    fn cache_clear_remote_store_is_safewrite() {
        // Remote stores are accepted under the recoverability argument
        // documented on check_laravel_cache_clear.
        assert_eq!(verdict("php artisan cache:clear --store=redis"), Verdict::Allowed(SafetyLevel::SafeWrite));
        assert_eq!(verdict("php artisan cache:clear --store=database"), Verdict::Allowed(SafetyLevel::SafeWrite));
    }

    #[test]
    fn cache_clear_unknown_flag_still_denied() {
        // Shape validation still rejects malformed invocations even
        // though the value allowlist is gone.
        assert_eq!(verdict("php artisan cache:clear --store=file --foo"), Verdict::Denied,);
    }

    #[test]
    fn cache_clear_file_is_safewrite() {
        assert_eq!(verdict("php artisan cache:clear --store=file"), Verdict::Allowed(SafetyLevel::SafeWrite));
    }

    #[test]
    fn cache_clear_array_is_safewrite() {
        assert_eq!(verdict("php artisan cache:clear --store=array"), Verdict::Allowed(SafetyLevel::SafeWrite));
    }

    #[test]
    fn please_cache_clear_file_is_safewrite() {
        assert_eq!(verdict("php please cache:clear --store=file"), Verdict::Allowed(SafetyLevel::SafeWrite));
    }

    #[test]
    fn php_help_is_inert() {
        assert_eq!(verdict("php --help"), Verdict::Allowed(SafetyLevel::Inert));
    }

    use crate::parse::Token;
    use proptest::prelude::*;

    // The 11 allowlisted ini directives, in the same order as `PHP_SAFE_INI_DIRECTIVES`. Proptest
    // samples one element at a time to drive the differential property against the static list.
    const PHP_SAFE_INI_KEYS: &[&str] = &[
        "date.timezone", "display_errors", "error_reporting", "max_execution_time", "max_input_time", "max_input_vars", "memory_limit",
        "opcache.enable", "opcache.enable_cli", "post_max_size", "upload_max_filesize",
    ];

    proptest! {
        // METAMORPHIC (value-side irrelevant). `is_safe_ini_pair` returns true iff the KEY half of
        // a `KEY=VALUE` string is in the static allowlist; the function never inspects what comes
        // after the first `=`. So two pairs that share a KEY must agree, regardless of how the
        // values differ — including when one side has no `=` and the other has a long value, when
        // the value itself contains further `=` signs (the first `=` is the split), and when the
        // value is empty. The oracle is the SAME function called twice; the property is that the
        // call's result depends only on KEY.
        #[test]
        fn is_safe_ini_pair_only_depends_on_the_key_half(
            key in proptest::sample::select(PHP_SAFE_INI_KEYS.to_vec()),
            v1 in "[a-zA-Z0-9._/+-]{0,12}",
            v2 in "[a-zA-Z0-9._/+]{0,12}",
        ) {
            let a = super::is_safe_ini_pair(&format!("{key}={v1}"));
            let b = super::is_safe_ini_pair(&format!("{key}={v2}"));
            proptest::prop_assert_eq!(a, b,
                "is_safe_ini_pair must agree on shared KEY {} regardless of value side", key);
        }

        // DIFFERENTIAL (oracle = the explicit allowlist). For each allowlisted KEY the pair must
        // be safe; for a KEY drawn from the SAME alphanumeric+`.`/`-`/`_` alphabet but guaranteed
        // not to be in the list, the pair must be denied. This is differential because the
        // reference is the literal allowlist declared above — not a property of `is_safe_ini_pair`
        // itself — and pins the function to that allowlist as the registry evolves.
        #[test]
        fn is_safe_ini_pair_matches_the_allowlist_exactly(
            safe_key in proptest::sample::select(PHP_SAFE_INI_KEYS.to_vec()),
            unsafe_key in "[a-z][a-z0-9._-]{0,10}",
            value in "[a-zA-Z0-9]{0,6}",
        ) {
            // The unsafe_key strategy overlaps the allowlist alphabet; assume it sampled a key NOT
            // in the list so we are genuinely comparing safe vs unsafe branches.
            proptest::prop_assume!(!PHP_SAFE_INI_KEYS.contains(&unsafe_key.as_str()));
            proptest::prop_assert!(
                super::is_safe_ini_pair(&format!("{safe_key}={value}")),
                "allowlisted key {} must be safe",
                safe_key,
            );
            proptest::prop_assert!(
                !super::is_safe_ini_pair(&format!("{unsafe_key}={value}")),
                "non-allowlisted key {} must be denied",
                unsafe_key,
            );
        }

        // INVARIANT (no `=` ⇒ denied). The first thing the function does is `split_once('=')`;
        // a missing `=` short-circuits to `false` before the allowlist is even consulted. Every
        // string without `=` therefore lands on the same branch — the value side cannot rescue it.
        // This pins the missing-equals rejection independently of the allowlist membership.
        #[test]
        fn is_safe_ini_pair_rejects_any_string_without_equals(
            s in "[a-zA-Z0-9._/ +-]{1,16}",
        ) {
            proptest::prop_assume!(!s.contains('='));
            proptest::prop_assert!(
                !super::is_safe_ini_pair(&s),
                "string {:?} without = must be denied, got accepted",
                s,
            );
        }

        // INVARIANT (two positionals ⇒ denied). `check_laravel_cache_clear` accepts at most one
        // bare positional after `cache:clear` — a second one trips the catch-all `Denied` branch
        // at the bottom of the loop. The handler is invoked with a slice that begins with the
        // sub-command itself (`cache:clear`) at index 0, then the cache:clear args; here `cache:clear`
        // is consumed as the one allowed positional, so adding a second non-flag token after it
        // must trip the deny. `mid_flags` are well-formed inert flags that the walker consumes
        // without bumping `saw_positional`; they cannot rescue the second positional, which the
        // loop always sees.
        #[test]
        fn check_laravel_cache_clear_denies_two_positionals(
            pos1 in "[a-z][a-z0-9._-]{0,10}",
            pos2 in "[a-z][a-z0-9._-]{0,10}",
            mid_flags in proptest::collection::vec(
                proptest::sample::select(&[
                    "--help", "-h", "--store=x", "--tags=y",
                ][..]),
                0..=3,
            ),
        ) {
            proptest::prop_assume!(!pos1.starts_with('-'));
            proptest::prop_assume!(!pos2.starts_with('-'));
            proptest::prop_assume!(pos1 != pos2 || mid_flags.is_empty());
            let mut tokens: Vec<Token> = Vec::with_capacity(3 + mid_flags.len());
            tokens.push(Token::from_test("cache:clear"));
            tokens.push(Token::from_test(&pos1));
            for f in &mid_flags {
                tokens.push(Token::from_test(f));
            }
            tokens.push(Token::from_test(&pos2));
            proptest::prop_assert_eq!(
                super::check_laravel_cache_clear(&tokens),
                Verdict::Denied,
                "two positionals {:?},{:?} with mid flags {:?} must be denied",
                pos1, pos2, mid_flags,
            );
        }

        // IDEMPOTENCE (inert flags leave SafeWrite unchanged). The walker treats `--help`/`-h` as
        // a no-op advance and consumes well-formed `--store=X`/`--tags=X`/`--store X`/`--tags X`
        // without changing the verdict. So a base input that already returns `SafeWrite` keeps
        // returning `SafeWrite` no matter which of those inert flags you append to it (any
        // subset, any order). The base is built to be SafeWrite: `cache:clear` plus one
        // positional; the property then sweeps every inert insertion and confirms the verdict
        // is unchanged.
        #[test]
        fn check_laravel_cache_clear_safe_write_survives_inert_flag_insertion(
            pos in "[a-z][a-z0-9._-]{0,8}",
            extras in proptest::collection::vec(
                proptest::sample::select(&[
                    "--help",
                    "-h",
                    "--store=file",
                    "--tags=foo",
                    "--store=redis",
                    "--tags=bar",
                ][..]),
                0..=4,
            ),
        ) {
            proptest::prop_assume!(!pos.starts_with('-'));
            let mut tokens: Vec<Token> = Vec::with_capacity(2 + extras.len());
            tokens.push(Token::from_test("cache:clear"));
            tokens.push(Token::from_test(&pos));
            for f in &extras {
                tokens.push(Token::from_test(f));
            }
            proptest::prop_assert_eq!(
                super::check_laravel_cache_clear(&tokens),
                Verdict::Allowed(SafetyLevel::SafeWrite),
                "base [cache:clear, {:?}] with extras {:?} must stay SafeWrite",
                pos, extras,
            );
        }
    }
}
