use crate::parse::{Token, WordSet};
use crate::verdict::{SafetyLevel, Verdict};

static PHP_BARE_FLAGS: WordSet = WordSet::new(&[
    "--help", "--info", "--ini", "--modules", "--version",
    "-V", "-h", "-i", "-m", "-v",
]);

static PHP_SAFE_INI_DIRECTIVES: WordSet = WordSet::new(&[
    "date.timezone",
    "display_errors",
    "error_reporting",
    "max_execution_time",
    "max_input_time",
    "max_input_vars",
    "memory_limit",
    "opcache.enable",
    "opcache.enable_cli",
    "post_max_size",
    "upload_max_filesize",
]);

static PHP_DELEGATE_SUBS: WordSet = WordSet::new(&["artisan", "please"]);

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

    if i + 1 == tokens.len() && PHP_BARE_FLAGS.contains(arg_str) {
        return Verdict::Allowed(SafetyLevel::Inert);
    }

    let basename = std::path::Path::new(arg_str)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(arg_str);

    if !PHP_DELEGATE_SUBS.contains(basename) {
        return Verdict::Denied;
    }

    let mut parts: Vec<&str> = Vec::with_capacity(tokens.len() - i);
    parts.push(basename);
    for t in &tokens[i + 1..] {
        parts.push(t.as_str());
    }
    let inner = shell_words::join(parts);
    crate::command_verdict(&inner)
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
        php_path_please: "php /Users/me/projects/clce.org/please stache:clear",
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
        assert_eq!(
            verdict("php artisan cache:clear file"),
            Verdict::Allowed(SafetyLevel::SafeWrite)
        );
    }

    #[test]
    fn cache_clear_bare_is_safewrite() {
        assert_eq!(
            verdict("php artisan cache:clear"),
            Verdict::Allowed(SafetyLevel::SafeWrite)
        );
    }

    #[test]
    fn cache_clear_remote_store_is_safewrite() {
        // Remote stores are accepted under the recoverability argument
        // documented on check_laravel_cache_clear.
        assert_eq!(
            verdict("php artisan cache:clear --store=redis"),
            Verdict::Allowed(SafetyLevel::SafeWrite)
        );
        assert_eq!(
            verdict("php artisan cache:clear --store=database"),
            Verdict::Allowed(SafetyLevel::SafeWrite)
        );
    }

    #[test]
    fn cache_clear_unknown_flag_still_denied() {
        // Shape validation still rejects malformed invocations even
        // though the value allowlist is gone.
        assert_eq!(
            verdict("php artisan cache:clear --store=file --foo"),
            Verdict::Denied,
        );
    }

    #[test]
    fn cache_clear_file_is_safewrite() {
        assert_eq!(
            verdict("php artisan cache:clear --store=file"),
            Verdict::Allowed(SafetyLevel::SafeWrite)
        );
    }

    #[test]
    fn cache_clear_array_is_safewrite() {
        assert_eq!(
            verdict("php artisan cache:clear --store=array"),
            Verdict::Allowed(SafetyLevel::SafeWrite)
        );
    }

    #[test]
    fn please_cache_clear_file_is_safewrite() {
        assert_eq!(
            verdict("php please cache:clear --store=file"),
            Verdict::Allowed(SafetyLevel::SafeWrite)
        );
    }

    #[test]
    fn php_help_is_inert() {
        assert_eq!(
            verdict("php --help"),
            Verdict::Allowed(SafetyLevel::Inert)
        );
    }
}
