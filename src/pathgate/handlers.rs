//! Operation-aware path gates: a command whose positional roles depend on a mode selector its own
//! grammar carries. Declared in `pathgates.toml` as `handler = "name"`; the fn reads the tokens and
//! gates each path by the role its operation implies. Every name here is asserted reachable from the
//! TOML (and vice-versa) by `pathgate_handler_names_resolve` — an unknown name is a config bug, not
//! a silent fail-open.

use super::{Role, gate};
use crate::parse::Token;

pub(crate) mod jj;
mod jq;
mod plist;

/// Names known to `dispatch` — the test guard checks the TOML uses exactly these.
#[cfg(test)]
pub(super) const NAMES: &[&str] = &[
    "ar_archive", "defaults_domain", "exiftool_mode", "jj_destination", "jj_repository", "jq_program", "jupytext_mode", "mtree_mode",
    "ncu_mode", "plutil_mode", "rdfind_mode", "textutil_mode", "tsc_response_file", "xattr_mode", "xcodebuild_settings", "yq_program",
];

pub(super) fn dispatch(name: &str, tokens: &[Token]) -> bool {
    match name {
        "ar_archive" => ar_archive(tokens),
        "defaults_domain" => plist::defaults_domain(tokens),
        "exiftool_mode" => exiftool_mode(tokens),
        "jj_destination" => jj::jj_destination(tokens),
        "jj_repository" => jj::jj_repository(tokens),
        "jq_program" => jq::jq_program(tokens),
        "jupytext_mode" => jupytext_mode(tokens),
        "mtree_mode" => mtree_mode(tokens),
        "ncu_mode" => ncu_mode(tokens),
        "plutil_mode" => plist::plutil_mode(tokens),
        "rdfind_mode" => rdfind_mode(tokens),
        "textutil_mode" => textutil_mode(tokens),
        "tsc_response_file" => tsc_response_file(tokens),
        "xattr_mode" => xattr_mode(tokens),
        "xcodebuild_settings" => xcodebuild_settings(tokens),
        "yq_program" => jq::yq_program(tokens),
        // Unreachable in practice (guarded by pathgate_handler_names_resolve). Fail CLOSED on a
        // misconfigured name so a typo can never silently ungate a command.
        _ => true,
    }
}

/// `ar KEYS ARCHIVE [MEMBERS…]` — the key-letter operation sets the archive's role: r/q/d/m/s
/// MUTATE the archive (write), t/p/x READ it (x extracts to cwd, a separate traversal concern).
/// The add operations r/q also read their member files (a disclosing read). KEYS is the first
/// token, either bare (`ar rcs`) or dash-led (`ar -rcs`); `--plugin`/`--target` take a value.
fn ar_archive(tokens: &[Token]) -> bool {
    let mut positionals: Vec<&str> = Vec::new();
    let mut keys: Option<&str> = None;
    let mut it = tokens[1..].iter().map(Token::as_str);
    while let Some(t) = it.next() {
        if t == "--plugin" || t == "--target" {
            it.next(); // consume the flag value so it is not mistaken for KEYS/archive
            continue;
        }
        if let Some(rest) = t.strip_prefix('-') {
            if keys.is_none() && !t.starts_with("--") && !rest.is_empty() {
                keys = Some(rest); // `-rcs` dash form of the key letters
            }
            continue; // any other flag never names a path
        }
        if keys.is_none() {
            keys = Some(t); // bare `rcs` key letters
            continue;
        }
        positionals.push(t);
    }
    let key_bytes = keys.map(str::as_bytes).unwrap_or_default();
    let op = key_bytes.iter().copied().find(u8::is_ascii_alphabetic);
    // The a/b/i positioning modifiers insert relative to a NAMED member, which appears BEFORE the
    // archive (`ar rb existing.o lib.a new.o`) — skip it, or the archive (the real write target)
    // would go ungated.
    let archive_idx = usize::from(key_bytes.iter().any(|b| matches!(b, b'a' | b'b' | b'i')));
    let Some(archive) = positionals.get(archive_idx) else { return false };
    let archive_role = match op {
        Some(b'r' | b'q' | b'd' | b'm' | b's') => Role::Write,
        _ => Role::Read, // t / p / x read the archive
    };
    if gate(archive_role, archive) {
        return true;
    }
    // r/q archive real files given as members — a sensitive member is a disclosing read.
    matches!(op, Some(b'r' | b'q')) && positionals.iter().skip(archive_idx + 1).any(|m| gate(Role::Read, m))
}

/// `tsc @FILE` — a RESPONSE FILE: tsc opens FILE and splices its contents in as arguments.
///
/// The declarative gate cannot see this, because the token it judges is `@/path`, and `@/path`
/// is not the path — the tool strips the `@`. The shields that match on a NAME segment
/// (`.ssh`, `.npmrc`) still fired through the prefix, which is what made the gap easy to miss;
/// the ones anchored to a location (`~/.cargo/credentials`, `~/.m2/settings.xml`,
/// `~/.gradle/gradle.properties`, `~/.composer/auth.json`, `~/.gem/credentials`, `~/.azure/`)
/// did not, and all six admitted `tsc @<that file>`.
///
/// It discloses: tsc reports each token it cannot resolve as `error TS6231: Could not resolve
/// the path 'X'`, so the file comes back a word at a time. Unlike the `--pretty` quoting this
/// command's other gate covers, this needs no flag at all. Measured with a canary.
///
/// Gates every `@`-prefixed argument, not only positionals: tsc accepts one anywhere on the
/// line. Composes with tsc's co-located `[command.path_gate]` rather than replacing it —
/// `should_deny` ORs the central and co-located gates, so the flag/positional roles stay
/// declared as data in `commands/tools/tsc.toml`.
fn tsc_response_file(tokens: &[Token]) -> bool {
    tokens[1..].iter().filter_map(|t| t.as_str().strip_prefix('@')).any(|path| gate(Role::Read, path))
}

/// `xattr [-lrsvx] [-p NAME | -w NAME VALUE | -d NAME | -c] file…` — the extended-attribute
/// operation sets the files' role: `-w`/`-d`/`-c` MUTATE each file's attributes (write),
/// everything else (a bare listing, or `-p NAME`) reads them.
///
/// Operation-aware rather than a blanket `positional = "write"` because the read form is the
/// common one — checking `com.apple.quarantine` on a download — and write-gating it would
/// over-deny every inspection of a file outside the workspace. The write form is the one that
/// matters: `xattr -w com.apple.quarantine … ~/.ssh/id_rsa` auto-approved before this.
///
/// A BARE listing is not gated at all, which follows this file's standing policy rather than
/// inventing one: metadata-only commands (`ls`, `stat`, `file`, `du`) are deliberately excluded
/// because they reveal names and sizes, not content. `xattr FILE` prints attribute NAMES and is
/// exactly that shape; `-p NAME` and `-l` print attribute VALUES, which is content, so those
/// read-gate like `cat` does.
///
/// The valued flags consume their operands so a NAME or VALUE is never mistaken for a file:
/// `-w` takes two, `-p`/`-d` take one.
fn xattr_mode(tokens: &[Token]) -> bool {
    let args: Vec<&str> = tokens[1..].iter().map(Token::as_str).collect();
    let writes = args.iter().any(|a| matches!(*a, "-w" | "-d" | "-c"));
    let reads_values = args.iter().any(|a| matches!(*a, "-p" | "-l"));
    if !writes && !reads_values {
        return false; // name-only listing: metadata, not content
    }
    let role = if writes { Role::Write } else { Role::Read };
    let mut it = args.iter().copied();
    while let Some(t) = it.next() {
        if t == "-w" {
            it.next();
            it.next();
            continue;
        }
        if t == "-p" || t == "-d" {
            it.next();
            continue;
        }
        if t.starts_with('-') {
            continue;
        }
        if gate(role, t) {
            return true;
        }
    }
    false
}

/// `exiftool [-TAG=VALUE …] files…` — a tag ASSIGNMENT rewrites the file's metadata in place;
/// without one, exiftool READS its operands and prints what it finds, which for `-v`, `-b` and
/// `-htmlDump` includes the bytes themselves. So a read gates every bare token as a read (an
/// option value such as a date format reads as a worktree name and passes), and `-r`/`-recurse`
/// makes each a tree. Read-gating once over-denied home inspection, which is why it was deferred;
/// home reads open now, so `exiftool ~/photo.jpg` passes and only the credential shields refuse.
///
/// Detecting the write is the whole difficulty, because exiftool's writing syntax IS its flag
/// syntax: `-TAG=VALUE` assigns, and `-all=` DELETES every tag. So any dash-led token carrying
/// `=` is treated as a write. That over-matches rather than under-matches (a read-only run with
/// an `=` in some option would merely gate its paths more strictly), which is the safe
/// direction for a detector whose miss is an ungated write.
fn exiftool_mode(tokens: &[Token]) -> bool {
    const VALUED: &[&str] = &["-o", "-tagsfromfile", "-api", "-charset", "-lang", "-@"];
    let args: Vec<&str> = tokens[1..].iter().map(Token::as_str).collect();
    let assigns = args.iter().any(|a| a.starts_with('-') && a.contains('=') && !VALUED.contains(a));
    let overwrites = args
        .iter()
        .any(|a| matches!(*a, "-overwrite_original" | "-overwrite_original_in_place" | "-delete_original"));
    if !assigns && !overwrites {
        let role = if args.iter().any(|a| a.eq_ignore_ascii_case("-r") || a.eq_ignore_ascii_case("-recurse")) {
            Role::ReadTree
        } else {
            Role::Read
        };
        return args.iter().any(|a| !(a.starts_with('-') && *a != "-") && gate(role, a));
    }
    let mut it = args.iter().copied();
    while let Some(t) = it.next() {
        if t == "-o" {
            if let Some(v) = it.next()
                && gate(Role::Write, v)
            {
                return true;
            }
            continue;
        }
        if VALUED.contains(&t) {
            it.next(); // a non-path option value
            continue;
        }
        if t.starts_with('-') {
            continue;
        }
        if gate(Role::Write, t) {
            return true;
        }
    }
    false
}

/// `rdfind [-action true] dir…` — the action flags decide whether the scanned trees are read or
/// destroyed. Per its own description: by default it reports duplicates and writes `results.txt`
/// in the CWD; `-makesymlinks`/`-makehardlinks`/`-deleteduplicates` replace or REMOVE duplicates
/// in the trees given as positionals; `-dryrun` previews without acting.
///
/// So the positionals are a write-target only when an action is actually enabled — the flags
/// take an explicit `true`/`false`, and `-dryrun true` disarms all of them. A plain scan of
/// `~/Pictures` stays allowed; `rdfind -deleteduplicates true ~/.ssh` does not.
fn rdfind_mode(tokens: &[Token]) -> bool {
    const ACTIONS: &[&str] = &["-makesymlinks", "-makehardlinks", "-deleteduplicates"];
    let args: Vec<&str> = tokens[1..].iter().map(Token::as_str).collect();
    let enabled = |flag: &str| args.windows(2).any(|w| w[0] == flag && w[1] == "true");
    let acting = ACTIONS.iter().any(|f| enabled(f));
    if !acting || enabled("-dryrun") {
        return false; // scan-and-report, or explicitly disarmed
    }
    let mut it = args.iter().copied();
    while let Some(t) = it.next() {
        if t.starts_with('-') {
            it.next(); // every rdfind option takes an explicit true/false or numeric value
            continue;
        }
        if gate(Role::Write, t) {
            return true;
        }
    }
    false
}

/// `mtree [-uUr] -p PATH` — verifies a file hierarchy against a spec, and can CHANGE it to match.
///
/// The dangerous flag is `-r`: it REMOVES every file in the tree that the spec does not mention,
/// so `mtree -r -p ~/.ssh` is mass deletion of a credential directory, and it auto-approved.
/// `-u`/`-U` modify the hierarchy (permissions, ownership, missing entries) to match.
///
/// The tree is a FLAG value (`-p`), never a positional, which is why every positional-shaped
/// sweep missed this one. `-f SPEC` and `-X EXCLUDE` are reads whatever the mode.
fn mtree_mode(tokens: &[Token]) -> bool {
    // ONLY genuinely valued flags. `-P` (do not follow symlinks) and `-L` (follow them) are
    // BOOLEAN, and listing them here was a live bypass: the walk consumed the following `-p` as
    // their value, so `mtree -P -p ~/.ssh -r` left the tree ungated while `mtree -r -p ~/.ssh`
    // denied — the same destructive operation, reordered. Asserting an arity without checking it
    // is the same defect this gate exists to catch.
    const VALUED: &[&str] = &["-f", "-K", "-k", "-p", "-s", "-N", "-X", "-R"];
    let args: Vec<&str> = tokens[1..].iter().map(Token::as_str).collect();
    let writes = args.iter().any(|a| matches!(*a, "-u" | "-U" | "-r"));
    let mut it = args.iter().copied();
    while let Some(t) = it.next() {
        if t == "-p" {
            let role = if writes { Role::Write } else { Role::Read };
            if let Some(v) = it.next()
                && gate(role, v)
            {
                return true;
            }
            continue;
        }
        if t == "-f" || t == "-X" {
            if let Some(v) = it.next()
                && gate(Role::Read, v)
            {
                return true;
            }
            continue;
        }
        if VALUED.contains(&t) {
            it.next();
        }
    }
    false
}

/// `ncu [--upgrade] [--packageFile FILE]` — npm-check-updates REPORTS available updates by
/// default and only rewrites the manifest with `--upgrade`/`-u`, so the manifest's role follows
/// the mode. Without this, `ncu --upgrade --packageFile /etc/package.json` wrote outside the
/// workspace.
fn ncu_mode(tokens: &[Token]) -> bool {
    let args: Vec<&str> = tokens[1..].iter().map(Token::as_str).collect();
    let writes = args.iter().any(|a| matches!(*a, "--upgrade" | "-u"));
    let role = if writes { Role::Write } else { Role::Read };
    let mut it = args.iter().copied();
    while let Some(t) = it.next() {
        if t == "--packageFile"
            && let Some(v) = it.next()
            && gate(role, v)
        {
            return true;
        }
    }
    false
}

/// `jupytext [--sync|--set-formats|--update-metadata|--to FMT] notebooks…` — the operation
/// decides whether the notebooks are read or REWRITTEN. `--sync` and `--set-formats` mutate the
/// notebook and its paired file in place; `--to` writes a converted sibling; a plain invocation
/// only inspects. `jupytext --sync ~/.ssh/config` auto-approved before this.
fn jupytext_mode(tokens: &[Token]) -> bool {
    const VALUED: &[&str] = &["--to", "--from", "--set-formats", "--output", "-o", "--pipe"];
    let args: Vec<&str> = tokens[1..].iter().map(Token::as_str).collect();
    let writes = args
        .iter()
        .any(|a| matches!(*a, "--sync" | "--set-formats" | "--update-metadata" | "--to" | "-o" | "--output"));
    let role = if writes { Role::Write } else { Role::Read };
    let mut it = args.iter().copied();
    while let Some(t) = it.next() {
        if t == "--output" || t == "-o" {
            if let Some(v) = it.next()
                && gate(Role::Write, v)
            {
                return true;
            }
            continue;
        }
        if VALUED.contains(&t) {
            it.next(); // a format name, not a path
            continue;
        }
        if t.starts_with('-') {
            continue;
        }
        if gate(role, t) {
            return true;
        }
    }
    false
}

/// `textutil -MODE [opts] files…` — `-convert`/`-strip` WRITE (to `-output`/`-outputdir`, else a
/// sibling of each input, so the input's directory is written); `-info`/`-cat` READ the inputs.
/// `-output`/`-outputdir` are always write targets.
fn textutil_mode(tokens: &[Token]) -> bool {
    const VALUED: &[&str] = &["-format", "-encoding", "-extension", "-fontname", "-fontsize", "-inputencoding", "-output", "-outputdir"];
    let args: Vec<&str> = tokens[1..].iter().map(Token::as_str).collect();
    let writes = args.iter().any(|a| *a == "-convert" || *a == "-strip");
    let has_output = args.iter().any(|a| *a == "-output" || *a == "-outputdir");
    // With no explicit output, a convert/strip writes each input's sibling → gate inputs as
    // write; otherwise (info/cat, or an explicit output flag) the inputs are read.
    let input_role = if writes && !has_output { Role::Write } else { Role::Read };
    let mut it = args.iter().copied();
    while let Some(t) = it.next() {
        if t == "-output" || t == "-outputdir" {
            if let Some(v) = it.next()
                && gate(Role::Write, v)
            {
                return true;
            }
            continue;
        }
        if VALUED.contains(&t) {
            it.next(); // consume a non-path flag value
            continue;
        }
        if t.starts_with('-') {
            continue; // a mode / standalone flag
        }
        if gate(input_role, t) {
            return true;
        }
    }
    false
}

/// `xcodebuild` takes actions and `KEY=VALUE` build settings as bare words. A setting that names
/// the compiler, the linker, or the install root runs or writes wherever that value says, and
/// `install` / `installsrc` write the product to `$DSTROOT`. Other settings and the ordinary
/// build actions are left to the command's own flag gate.
///
/// A valued flag's argument is skipped, so a scheme or target named `install` is not an action.
fn xcodebuild_settings(tokens: &[Token]) -> bool {
    // The valued flags from the command definition. A value is never an action or a setting.
    const VALUED: &[&str] = &[
        "-arch",
        "-archivePath",
        "-buildVersion",
        "-configuration",
        "-defaultLanguage",
        "-defaultPackageRegistryURL",
        "-derivedDataPath",
        "-destination",
        "-destination-timeout",
        "-exportArchive",
        "-exportFormat",
        "-exportLanguage",
        "-exportLocalizations",
        "-exportNotarizedApp",
        "-exportOptionsPlist",
        "-exportPath",
        "-find-executable",
        "-find-library",
        "-importComponent",
        "-importLocalizations",
        "-importPath",
        "-jobs",
        "-localizationPath",
        "-maximum-concurrent-test-device-destinations",
        "-maximum-concurrent-test-simulator-destinations",
        "-maximum-parallel-testing-workers",
        "-only-testing",
        "-only-testing:TEST-IDENTIFIER",
        "-onlyTesting",
        "-packageAuthorizationProvider",
        "-packageCachePath",
        "-packageDependencySCMToRegistryTransformation",
        "-packageFingerprintPolicy",
        "-packageSigningEntityPolicy",
        "-parallel-testing-enabled",
        "-parallel-testing-worker-count",
        "-prepareDeviceSupport",
        "-project",
        "-resultBundlePath",
        "-resultBundleVersion",
        "-resultStreamPath",
        "-retry-tests-on-failure",
        "-runDestination",
        "-runDestinationTimeout",
        "-scheme",
        "-scmProvider",
        "-sdk",
        "-skip-testing",
        "-skip-testing:TEST-IDENTIFIER",
        "-skipTesting",
        "-target",
        "-test-enumeration-format",
        "-test-enumeration-output-path",
        "-test-enumeration-style",
        "-test-iterations",
        "-test-repetition-relaunch-enabled",
        "-test-timeouts-enabled",
        "-testLanguage",
        "-testPlan",
        "-testProductsPath",
        "-testRegion",
        "-toolchain",
        "-workspace",
        "-xcconfig",
        "-xctestrun",
    ];
    let mut i = 1;
    while i < tokens.len() {
        let t = tokens[i].as_str();
        if t == "--" {
            return tokens[i + 1..].iter().any(|token| names_a_compiler_or_install(token.as_str()));
        }
        if let Some(glued) = valued_assignment(t, VALUED) {
            i += if glued { 1 } else { 2 };
            continue;
        }
        if t.starts_with('-') && t != "-" {
            i += 1;
            continue;
        }
        if names_a_compiler_or_install(t) {
            return true;
        }
        i += 1;
    }
    false
}

/// `Some(true)` when `token` is `flag=value`; `Some(false)` when it is a valued flag whose
/// value is the next token.
fn valued_assignment(token: &str, valued: &[&str]) -> Option<bool> {
    valued.iter().find_map(|flag| {
        if token == *flag { Some(false) } else { token.strip_prefix(flag).filter(|rest| rest.starts_with('=')).map(|_| true) }
    })
}

fn names_a_compiler_or_install(token: &str) -> bool {
    matches!(token, "install" | "installsrc")
        || token.split_once('=').is_some_and(|(key, _)| matches!(key, "CC" | "LD" | "SWIFT_EXEC" | "DSTROOT"))
}

#[cfg(test)]
mod tests {
    fn allows(command: &str) -> bool {
        crate::is_safe_command(command)
    }

    #[test]
    fn a_build_setting_that_names_a_compiler_or_an_install_root_asks() {
        for command in [
            "xcodebuild build CC=/tmp/y/evil",
            "xcodebuild build LD=/tmp/y/evil",
            "xcodebuild build SWIFT_EXEC=/tmp/y/evil",
            "xcodebuild build DSTROOT=/etc",
            "xcodebuild build DSTROOT=/etc install",
            "xcodebuild install",
            "xcodebuild installsrc",
            "xcodebuild -toolchain /tmp/y/Evil.xctoolchain build",
            "xcodebuild -xcconfig /tmp/y/evil.xcconfig build",
        ] {
            assert!(!allows(command), "{command}");
        }
        for command in [
            "xcodebuild -project Foo.xcodeproj -scheme Foo -configuration Debug -destination 'platform=macOS' build CODE_SIGNING_ALLOWED=NO CODE_SIGNING_REQUIRED=NO CODE_SIGN_IDENTITY=\"\"",
            "xcodebuild -workspace Foo.xcworkspace -scheme Foo -configuration Release archive -archivePath build/Foo.xcarchive",
            "xcodebuild -project Foo.xcodeproj -scheme Foo test -destination 'platform=iOS Simulator,name=iPhone 15' -enableCodeCoverage",
            "xcodebuild -project Foo.xcodeproj -scheme Foo clean",
            "xcodebuild -project Foo.xcodeproj -scheme Foo analyze",
            "xcodebuild -project Foo.xcodeproj -scheme Foo docbuild",
            "xcodebuild -scheme install build",
            "xcodebuild -destination 'platform=macOS' build",
            "xcodebuild -xcconfig Config.xcconfig build",
            "xcodebuild -toolchain Local.xctoolchain build",
            "xcodebuild build CODE_SIGNING_ALLOWED=NO",
        ] {
            assert!(allows(command), "{command}");
        }
    }
}
