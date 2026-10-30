use crate::parse::Token;
use crate::policy::{FlagSet, takes_next_word};
use crate::registry;
use crate::verdict::Verdict;

const LIBXO_OPTIONS: &[&str] = &[
    "color", "dtrt", "flush", "flush-line", "html", "info", "json", "keys", "log-gettext", "log-syslog", "no-color", "no-humanize",
    "no-locale", "no-retain", "no-top", "pretty", "retain", "text", "underscores", "units", "warn", "warn-xml", "xml", "xpath",
];

pub fn check_df(tokens: &[Token]) -> Verdict {
    if !libxo_values_are_known(tokens) {
        return Verdict::Denied;
    }
    registry::try_fallback_grammar("df", tokens).unwrap_or(Verdict::Denied)
}

fn is_libxo_value(value: &str) -> bool {
    !value.is_empty() && value.split(',').all(|part| LIBXO_OPTIONS.contains(&part))
}

fn libxo_values_are_known(tokens: &[Token]) -> bool {
    let Some((standalone, valued)) = registry::fallback_flag_sets("df") else {
        return false;
    };
    let mut i = 1;
    while i < tokens.len() {
        let t = tokens[i].as_str();
        i += 1;
        if t == "--" {
            break;
        }
        if let Some(long) = t.strip_prefix("--") {
            let (name, inline) = match long.split_once('=') {
                Some((name, value)) => (format!("--{name}"), Some(value)),
                None => (t.to_string(), None),
            };
            if name == "--libxo" {
                let value = inline.or_else(|| tokens.get(i).map(Token::as_str));
                if !value.is_some_and(is_libxo_value) {
                    return false;
                }
            } else if inline.is_none() && takes_next_word(standalone, valued, &name) {
                i += 1;
            }
            continue;
        }
        if !t.starts_with('-') {
            continue;
        }
        let bytes = t.as_bytes();
        for (j, &b) in bytes.iter().enumerate().skip(1) {
            if standalone.contains_short(b) {
                continue;
            }
            if valued.contains_short(b) && j + 1 == bytes.len() {
                i += 1;
            }
            break;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::{LIBXO_OPTIONS, is_libxo_value};

    fn check(cmd: &str) -> bool {
        crate::is_safe_command(cmd)
    }

    safe! {
        bare: "df",
        human_readable: "df -h",
        libxo_json_space: "df --libxo json .",
        libxo_xml_equals: "df --libxo=xml -h",
        libxo_comma_joined: "df --libxo json,pretty,warn .",
        libxo_html_xpath: "df --libxo=html,xpath,underscores /",
        libxo_after_double_dash_is_operand: "df -h -- --libxo=evil",
        libxo_as_value_of_type_flag: "df -t --libxo=evil",
        libxo_as_value_of_long_type_flag: "df --type --libxo=evil",
        alias_gdf_libxo_json: "gdf --libxo=json,pretty",
    }

    denied! {
        libxo_unknown_space: "df --libxo bogus .",
        libxo_unknown_equals: "df --libxo=bogus -h",
        libxo_encoder: "df --libxo=encoder=test .",
        libxo_colon_brief: "df --libxo:J .",
        libxo_mixed_valid_and_unknown: "df --libxo=json,bogus .",
        libxo_empty_equals: "df --libxo= .",
        libxo_trailing_comma: "df --libxo=json, .",
        libxo_leading_comma: "df --libxo=,json .",
        libxo_missing_value: "df --libxo",
        libxo_after_long_standalone: "df --human-readable --libxo=bogus",
        libxo_after_inline_long_valued: "df --type=apfs --libxo=bogus",
        libxo_after_glued_short_type: "df -tapfs --libxo=bogus",
        alias_gdf_libxo_unknown: "gdf --libxo=bogus",
    }

    proptest! {
        #[test]
        fn every_documented_libxo_combination_is_allowed(
            parts in prop::collection::vec(prop::sample::select(LIBXO_OPTIONS), 1..=4),
            glued in any::<bool>(),
        ) {
            let joined = parts.join(",");
            prop_assert!(is_libxo_value(&joined));
            let cmd = if glued {
                format!("df --libxo={joined} .")
            } else {
                format!("df --libxo {joined} .")
            };
            prop_assert!(crate::is_safe_command(&cmd), "{}", cmd);
        }

        #[test]
        fn any_unlisted_libxo_token_is_denied(
            bad in "[a-z0-9_-]{1,12}",
            with_prefix in any::<bool>(),
            glued in any::<bool>(),
        ) {
            prop_assume!(!LIBXO_OPTIONS.contains(&bad.as_str()));
            let value = if with_prefix {
                format!("json,{bad}")
            } else {
                bad
            };
            prop_assert!(!is_libxo_value(&value));
            let cmd = if glued {
                format!("df --libxo={value} .")
            } else {
                format!("df --libxo {value} .")
            };
            prop_assert!(!crate::is_safe_command(&cmd), "{}", cmd);
        }
    }
}
