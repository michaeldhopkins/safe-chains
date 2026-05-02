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

static CACHE_CLEAR_SAFE_STORES: WordSet = WordSet::new(&["array", "file", "null"]);

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

pub fn check_laravel_cache_clear(tokens: &[Token]) -> Verdict {
    if tokens.len() == 2 && matches!(tokens[1].as_str(), "--help" | "-h") {
        return Verdict::Allowed(SafetyLevel::Inert);
    }

    let mut store: Option<&str> = None;
    let mut saw_positional = false;
    let mut i = 1;
    while i < tokens.len() {
        let s = tokens[i].as_str();
        if s == "--help" || s == "-h" {
            i += 1;
            continue;
        }
        if s == "--store" {
            let Some(next) = tokens.get(i + 1) else {
                return Verdict::Denied;
            };
            store = Some(next.as_str());
            i += 2;
            continue;
        }
        if let Some(val) = s.strip_prefix("--store=") {
            store = Some(val);
            i += 1;
            continue;
        }
        if !s.starts_with('-') && !saw_positional {
            store = Some(s);
            saw_positional = true;
            i += 1;
            continue;
        }
        return Verdict::Denied;
    }

    let Some(store) = store else {
        return Verdict::Denied;
    };
    if !CACHE_CLEAR_SAFE_STORES.contains(store) {
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
        php_artisan_cache_clear_redis: "php artisan cache:clear --store=redis",
        php_artisan_cache_clear_database: "php artisan cache:clear --store=database",
        php_artisan_cache_clear_memcached: "php artisan cache:clear --store=memcached",
        php_artisan_cache_clear_dynamodb: "php artisan cache:clear --store=dynamodb",
        php_artisan_cache_clear_no_store: "php artisan cache:clear",
        php_artisan_cache_clear_with_tags: "php artisan cache:clear --tags=foo --store=file",
        php_artisan_cache_clear_unknown_flag: "php artisan cache:clear --store=file --foo",
        php_path_basename_only: "php /tmp/please-bak stache:clear",
        php_artisan_cache_clear_positional_redis: "php artisan cache:clear redis",
        php_artisan_cache_clear_positional_glide: "php artisan cache:clear glide",
        php_artisan_cache_clear_positional_database: "php artisan cache:clear database",
        php_artisan_cache_clear_two_positionals: "php artisan cache:clear file array",
        php_artisan_cache_clear_positional_then_unsafe_flag: "php artisan cache:clear file --tags=foo",
    }

    #[test]
    fn cache_clear_positional_file_is_safewrite() {
        assert_eq!(
            verdict("php artisan cache:clear file"),
            Verdict::Allowed(SafetyLevel::SafeWrite)
        );
    }

    #[test]
    fn cache_clear_positional_glide_denied() {
        assert_eq!(verdict("php artisan cache:clear glide"), Verdict::Denied);
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
