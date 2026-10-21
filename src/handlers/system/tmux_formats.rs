//! Which names a tmux format reads.
//!
//! tmux expands `#{name}` from its own format variables and options, and a name that is neither
//! falls back to the server's environment: `tmux display -p '#{OPENAI_API_KEY}'` prints the key
//! (measured on tmux 3.8). So a format passes only when every name it looks up is a format variable
//! from the tmux manual, a user option (`@name`), or an option name (`status-left`, with a hyphen no
//! environment variable carries). The condition of `#{?…}` and the operand of a modifier such as
//! `t:`, `=5:` or `s/a/b/:` are looked up; the operands of the comparisons, `l:`, the loops and
//! the other modifiers that take text are expanded as formats, so only the formats inside them are
//! checked.

static VARIABLES: &[&str] = &[
    "active_window_index", "alternate_on", "alternate_saved_x", "alternate_saved_y", "bracket_paste_flag", "buffer_created", "buffer_full",
    "buffer_name", "buffer_sample", "buffer_size", "client_activity", "client_cell_height", "client_cell_width", "client_colours",
    "client_control_mode", "client_created", "client_discarded", "client_flags", "client_height", "client_key_table",
    "client_last_session", "client_name", "client_pid", "client_prefix", "client_readonly", "client_session", "client_termfeatures",
    "client_termname", "client_termtype", "client_tty", "client_uid", "client_user", "client_utf8", "client_width", "client_written",
    "command", "command_list_alias", "command_list_name", "command_list_usage", "config_files", "copy_cursor_hyperlink",
    "copy_cursor_line", "copy_cursor_word", "copy_cursor_x", "copy_cursor_y", "copy_line_numbers", "copy_position", "copy_position_limit",
    "current_file", "cursor_blinking", "cursor_character", "cursor_colour", "cursor_flag", "cursor_shape", "cursor_very_visible",
    "cursor_x", "cursor_y", "history_added", "history_bytes", "history_collected", "history_generation", "history_limit", "history_size",
    "hook", "hook_client", "hook_fire_count", "hook_fire_time", "hook_last", "hook_monitor_format", "hook_monitor_target", "hook_pane",
    "hook_session", "hook_session_name", "hook_value", "hook_window", "hook_window_index", "hook_window_name", "host", "host_short",
    "insert_flag", "key_command", "key_has_repeat", "key_note", "key_prefix", "key_repeat", "key_string", "key_string_width", "key_table",
    "key_table_width", "keypad_cursor_flag", "keypad_flag", "last_window_index", "line", "loop_index", "loop_last_flag", "mouse_all_flag",
    "mouse_any_flag", "mouse_button_flag", "mouse_hyperlink", "mouse_line", "mouse_sgr_flag", "mouse_standard_flag", "mouse_status_line",
    "mouse_status_range", "mouse_utf8_flag", "mouse_word", "mouse_x", "mouse_y", "next_session_id", "next_window_active",
    "next_window_index", "option_array_key", "option_has_array_key", "option_has_value", "option_is_array", "option_is_hook",
    "option_is_parent", "option_is_string", "option_is_user", "option_name", "option_value", "option_value_only", "origin_flag",
    "pane_active", "pane_at_bottom", "pane_at_left", "pane_at_right", "pane_at_top", "pane_bg", "pane_bottom", "pane_command_duration",
    "pane_command_end_time", "pane_command_running", "pane_command_start_time", "pane_command_status", "pane_current_command",
    "pane_current_path", "pane_dead", "pane_dead_signal", "pane_dead_status", "pane_dead_time", "pane_fg", "pane_flags",
    "pane_floating_flag", "pane_format", "pane_height", "pane_id", "pane_in_mode", "pane_index", "pane_input_off", "pane_key_mode",
    "pane_last", "pane_last_output_time", "pane_last_prompt_time", "pane_left", "pane_marked", "pane_marked_set", "pane_modal_flag",
    "pane_mode", "pane_output_generation", "pane_path", "pane_pb_progress", "pane_pb_state", "pane_pid", "pane_pipe", "pane_pipe_pid",
    "pane_private_modes", "pane_right", "pane_search_string", "pane_start_command", "pane_start_command_list", "pane_start_path",
    "pane_synchronized", "pane_tabs", "pane_title", "pane_top", "pane_tty", "pane_unseen_changes", "pane_unzoomed_height",
    "pane_unzoomed_width", "pane_width", "pane_x", "pane_y", "pane_z", "pane_zoomed_flag", "pid", "popup_centre_x", "popup_centre_y",
    "popup_height", "popup_last_x", "popup_last_y", "popup_mouse_bottom", "popup_mouse_centre_x", "popup_mouse_centre_y",
    "popup_mouse_top", "popup_mouse_x", "popup_mouse_y", "popup_pane_bottom", "popup_pane_left", "popup_pane_right", "popup_pane_top",
    "popup_status_line_y", "popup_width", "popup_window_status_line_x", "popup_window_status_line_y", "prev_window_active",
    "prev_window_index", "rectangle_toggle", "refresh_active", "scroll_position", "scroll_region_lower", "scroll_region_upper",
    "search_count", "search_count_partial", "search_match", "search_present", "selection_active", "selection_end_x", "selection_end_y",
    "selection_mode", "selection_present", "selection_start_x", "selection_start_y", "server_sessions", "session_active",
    "session_activity", "session_activity_flag", "session_alerts", "session_attached", "session_attached_list", "session_bell_flag",
    "session_created", "session_format", "session_group", "session_group_attached", "session_group_attached_list", "session_group_list",
    "session_group_many_attached", "session_group_size", "session_grouped", "session_id", "session_last_attached", "session_many_attached",
    "session_marked", "session_name", "session_path", "session_silence_flag", "session_stack", "session_windows", "sixel_support",
    "socket_path", "start_time", "synchronized_output_flag", "uid", "user", "version", "window_active", "window_active_clients",
    "window_active_clients_list", "window_active_sessions", "window_active_sessions_list", "window_activity", "window_activity_flag",
    "window_bell_flag", "window_bigger", "window_cell_height", "window_cell_width", "window_end_flag", "window_flags", "window_format",
    "window_height", "window_id", "window_index", "window_last_flag", "window_layout", "window_linked", "window_linked_sessions",
    "window_linked_sessions_list", "window_manual_height", "window_manual_width", "window_marked_flag", "window_modal_pane", "window_name",
    "window_offset_x", "window_offset_y", "window_panes", "window_raw_flags", "window_silence_flag", "window_stack_index",
    "window_start_flag", "window_visible_layout", "window_width", "window_zoomed_flag", "wrap_flag",
];

static TAKE_TEXT: &[&str] =
    &["!", "!!", "!=", "&&", "<", "<=", "==", ">", ">=", "A", "C", "L", "N", "P", "R", "S", "W", "e", "l", "m", "||"];

pub(super) fn reads_only_tmux_values(word: &str) -> bool {
    let mut rest = word;
    while let Some(at) = rest.find("#{") {
        let Some((inner, after)) = braced(&rest[at + 2..]) else { return false };
        if !expression_reads_only_tmux_values(inner) {
            return false;
        }
        rest = after;
    }
    true
}

fn braced(text: &str) -> Option<(&str, &str)> {
    let mut depth = 0usize;
    let mut chars = text.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        match c {
            '#' if chars.peek().is_some_and(|(_, next)| *next == '{') => {
                chars.next();
                depth += 1;
            }
            '}' if depth == 0 => return Some((&text[..at], &text[at + 1..])),
            '}' => depth -= 1,
            _ => {}
        }
    }
    None
}

fn top_level(text: &str, separator: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        match c {
            '#' if chars.peek().is_some_and(|(_, next)| *next == '{') => {
                chars.next();
                depth += 1;
            }
            '}' => depth = depth.saturating_sub(1),
            c if c == separator && depth == 0 => {
                parts.push(&text[start..at]);
                start = at + c.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(&text[start..]);
    parts
}

fn expression_reads_only_tmux_values(inner: &str) -> bool {
    if let Some(choices) = inner.strip_prefix('?') {
        let parts = top_level(choices, ',');
        return parts.iter().enumerate().all(|(at, part)| {
            if at % 2 == 0 && at + 1 < parts.len() && !part.starts_with("#{") { is_a_tmux_name(part) } else { reads_only_tmux_values(part) }
        });
    }
    let parts = top_level(inner, ':');
    let (Some(modifiers), true) = (parts.first(), parts.len() > 1) else { return is_a_tmux_name(inner) };
    let operand = &inner[modifiers.len() + 1..];
    if modifiers.split(';').all(|m| TAKE_TEXT.contains(&operator(m))) {
        return reads_only_tmux_values(operand);
    }
    top_level(operand, ',').iter().all(|part| is_a_tmux_name(part))
}

fn operator(modifier: &str) -> &str {
    ["!!", "!=", "&&", "<=", "==", ">=", "||"]
        .into_iter()
        .find(|op| modifier.starts_with(op))
        .unwrap_or_else(|| modifier.get(..modifier.chars().next().map_or(0, char::len_utf8)).unwrap_or_default())
}

fn is_a_tmux_name(name: &str) -> bool {
    let option = |rest: &str| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'));
    VARIABLES.binary_search(&name).is_ok()
        || (!name.is_empty() && name.chars().all(|c| c.is_ascii_digit()))
        || name.strip_prefix('@').is_some_and(option)
        || (name.contains('-') && option(name) && name.starts_with(|c: char| c.is_ascii_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn the_variable_list_is_sorted_for_its_binary_search() {
        assert!(VARIABLES.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(TAKE_TEXT.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn a_format_naming_only_tmux_values_passes() {
        for word in [
            "#{session_name}", "#{t:window_activity}", "#{=10:pane_title}", "#{?pane_active,yes,no}", "#{?#{==:#{pane_index},0},first,}",
            "#{==:#{session_name},work}", "#{l:OPENAI_API_KEY}", "#{@my-option}", "#{status-left}", "#{s/a/b/:window_name}",
            "#{S:#{session_name} }", "plain text", "#{||:#{pane_active},#{window_active}}", "#{?0,a,pane_dead,c,d}",
        ] {
            assert!(reads_only_tmux_values(word), "{word}");
        }
    }

    #[test]
    fn a_format_that_could_read_the_environment_is_refused() {
        for word in [
            "#{OPENAI_API_KEY}", "#{HOME}", "#{=3:GITHUB_TOKEN}", "#{t:SECRET}", "#{s/a/b/:SECRET}", "#{?SECRET,a,b}",
            "#{?0,a,SECRET,c,d}", "#{==:#{SECRET},x}", "#{session_name}#{SECRET}", "#{#{session_name}}", "#{q:SECRET}", "#{b;=5:SECRET}",
            "#{session_name", "#{s/a:b/c/:SECRET}", "#{http_proxy}",
            "#{?0,#{session_name},SECRET,d}", "#{SECRET-KEY}", "#{a-b.c}", "#{a-b c}",
        ] {
            assert!(!reads_only_tmux_values(word), "{word}");
        }
    }

    proptest! {
        /// A name that is not a tmux variable, an option or a user option is refused in every
        /// position tmux looks a name up, however the format around it is spelled.
        #[test]
        fn an_unknown_name_is_refused_wherever_it_is_looked_up(
            name in "[A-Z][A-Z0-9_]{0,12}",
            shape in prop::sample::select(vec!["#{N}", "#{t:N}", "#{=5:N}", "#{?N,a,b}", "x#{N}y", "#{?pane_active,#{N},b}", "#{==:#{N},a}"]),
        ) {
            prop_assert!(!reads_only_tmux_values(&shape.replace('N', &name)), "{}", shape);
            prop_assert!(reads_only_tmux_values(&shape.replace('N', "session_name")), "{}", shape);
        }
    }
}
