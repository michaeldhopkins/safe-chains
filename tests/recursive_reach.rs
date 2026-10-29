//! A recursive write reaches everything below its operand, so it is judged by the write-frozen
//! metadata (`.git`, `.jj`, `.hg`, `.sl`, `.envrc`) it would walk into, at any depth.

use proptest::prelude::*;
use safe_chains::pathctx::PathCtx;
use safe_chains::{Verdict, command_verdict_in};

struct Project {
    _dir: tempfile::TempDir,
    root: String,
}

fn project(dirs: &[&str]) -> Project {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("scratch project");
    let root = dir.path().canonicalize().expect("canonical root").to_str().expect("utf-8 root").to_string();
    for sub in dirs {
        std::fs::create_dir_all(format!("{root}/{sub}")).expect("project folders");
    }
    std::fs::write(format!("{root}/src/a.txt"), "a").expect("project file");
    Project { _dir: dir, root }
}

fn climb_to(p: &Project, target: &std::path::Path) -> String {
    let depth = std::path::Path::new(&p.root).components().count() - 1;
    format!("{}{}", "../".repeat(depth), target.to_str().expect("utf-8").trim_start_matches('/'))
}

fn at(p: &Project, cmd: &str) -> Verdict {
    command_verdict_in(cmd, PathCtx { cwd: Some(p.root.clone()), root: Some(p.root.clone()), session_id: None })
}

const RECURSIVE_WRITES: [&str; 12] = [
    "chmod -R 755 BASE",
    "chmod -R u+x BASE",
    "chflags -R uchg BASE",
    "find BASE -exec chmod +x {} +",
    "find BASE -type f -exec sed -i s/a/b/ {} +",
    "fd -H . BASE -x chmod +x",
    "cp -r BASE backup",
    "rm -rf BASE",
    "rsync -a BASE/ backup",
    "ditto BASE backup",
    "find BASE -print0 | xargs -0 chmod +x",
    "find BASE -type f | while read f; do chmod +x \"$f\"; done",
];

#[test]
fn a_recursive_write_over_the_project_root_is_refused() {
    let p = project(&["src", ".git/hooks"]);
    for cmd in RECURSIVE_WRITES {
        let cmd = cmd.replace("BASE", ".");
        assert!(!at(&p, &cmd).is_allowed(), "{cmd} was approved");
    }
}

#[test]
fn copying_a_tree_that_holds_a_repository_over_the_project_is_refused() {
    let p = project(&["src", ".git/hooks", "other/.git/hooks"]);
    assert!(!at(&p, "cp -r other/. .").is_allowed());
    assert!(!at(&p, "cp -R other dest").is_allowed());
    for cmd in ["rsync -a other/ .", "rsync -r other dest", "ditto other .", "cp -a other/. ."] {
        assert!(!at(&p, cmd).is_allowed(), "{cmd} was approved");
    }
    for cmd in ["cp -r src dest", "rsync -a src/ dest", "ditto src dest"] {
        assert!(at(&p, cmd).is_allowed(), "{cmd} was refused");
    }
}

#[test]
fn a_recursive_write_over_a_plain_folder_still_approves() {
    let p = project(&["src/sub", ".git/hooks"]);
    for cmd in
        ["chmod -R 755 src", "chflags -R nouchg src", "find src -exec chmod +x {} +", "fd . -x chmod +x", "chmod 755 .", "rm -rf src"]
    {
        assert!(at(&p, cmd).is_allowed(), "{cmd} was refused");
    }
}

#[test]
fn a_traversal_that_only_reads_still_approves_over_the_root() {
    let p = project(&["src", ".git/hooks"]);
    for cmd in ["find . -exec cat {} +", "find . -name x -print", "fd -H . -x wc -l", "find . -print0 | xargs -0 wc -l"] {
        assert!(at(&p, cmd).is_allowed(), "{cmd} was refused");
    }
}

#[test]
fn a_tree_too_large_to_list_refuses_writes_and_keeps_reads() {
    let p = project(&["src", "big"]);
    for i in 0..20_001 {
        std::fs::write(format!("{}/big/{i}", p.root), "").expect("file");
    }
    assert!(!at(&p, "chmod -R 755 big").is_allowed());
    assert!(at(&p, "find big -exec cat {} +").is_allowed());
    assert!(at(&p, "rm -rf big").is_allowed());
}

#[test]
fn a_glob_operand_reaches_what_its_matches_hold() {
    let p = project(&["src", "vendor/lib/.git"]);
    assert!(!at(&p, "chmod -R 755 *").is_allowed());
    assert!(at(&p, "chmod -R 755 s*").is_allowed());
}

const FROZEN: [&str; 5] = [".git", ".jj", ".hg", ".sl", ".envrc"];

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// Wherever a frozen name sits below the operand, every recursive write over it is refused;
    /// the same tree without it is approved.
    #[test]
    fn a_frozen_name_at_any_depth_is_reached(
        depth in proptest::collection::vec("[a-z]{1,6}", 0..4),
        frozen in 0..FROZEN.len(),
        command in 0..RECURSIVE_WRITES.len(),
    ) {
        let parent = std::iter::once("work".to_string()).chain(depth).collect::<Vec<_>>().join("/");
        let p = project(&["src", parent.as_str()]);
        let cmd = RECURSIVE_WRITES[command].replace("BASE", "work");
        prop_assert!(at(&p, &cmd).is_allowed(), "{} refused with nothing frozen below", cmd);
        let name = format!("{}/{parent}/{}", p.root, FROZEN[frozen]);
        if FROZEN[frozen] == ".envrc" {
            std::fs::write(&name, "").expect("frozen file");
        } else {
            std::fs::create_dir_all(&name).expect("frozen folder");
        }
        prop_assert!(!at(&p, &cmd).is_allowed(), "{} approved with {} below", cmd, name);
    }
}

#[test]
fn a_recursive_operand_that_cannot_be_placed_is_refused() {
    let p = project(&["src", "sub/.git"]);
    for cmd in [
        "chmod -R 755 \"$(pwd)\"", "chmod -R 755 \"$(pwd)/src\"", "cp -r \"$(pwd)/sub/.\" dest",
        "for d in . ; do chmod -R 755 \"$d\"; done",
    ] {
        assert!(!at(&p, cmd).is_allowed(), "{cmd} was approved");
    }
    assert!(at(&p, "chmod -R 755 src").is_allowed());
}

#[test]
fn copying_a_metadata_folder_itself_is_refused() {
    let p = project(&["src", "sub/.git/hooks", "d"]);
    for cmd in ["rsync -a sub/.git d/", "rsync -r sub/.git d", "cp -r sub/.git d/"] {
        assert!(!at(&p, cmd).is_allowed(), "{cmd} was approved");
    }
    assert!(at(&p, "rsync -a sub/.git/hooks/ d/").is_allowed());
}

#[test]
fn a_traversal_that_prunes_the_metadata_folder_is_judged_without_it() {
    let p = project(&["src", ".git/hooks"]);
    for cmd in [
        "find . -path ./.git -prune -o -exec chmod +x {} +",
        "find . -path ./.git -prune -o -type f -exec sed -i s/a/b/ {} +",
        "find . \\( -name .git -o -name .jj \\) -prune -o -exec chmod +x {} +",
        "find . -type f -not -path './.git/*' -exec sed -i s/a/b/ {} +",
        "find . -type f ! -path '*/.git/*' -exec chmod 644 {} +",
        "find . -path ./.git -prune -o -print0 | xargs -0 chmod +x",
    ] {
        assert!(at(&p, cmd).is_allowed(), "{cmd} was refused");
    }
    for cmd in [
        "find . -path ./.git -prune -o -delete",
        "find . -not -path './.git/*' -exec chmod +x {} +",
        "find . -exec chmod +x {} + -not -path './.git/*'",
        "find . -name '*.txt' -exec sed -i s/a/b/ {} +",
        "find . -path ./.gi -prune -o -exec chmod +x {} +",
        "find . ! -path ./.git -prune -o -exec chmod +x {} +",
    ] {
        assert!(!at(&p, cmd).is_allowed(), "{cmd} was approved");
    }
    let nested = project(&["src", ".git/hooks", "sub/.git/hooks"]);
    assert!(!at(&nested, "find . -path ./.git -prune -o -exec chmod +x {} +").is_allowed());
    assert!(at(&nested, "find . -name .git -prune -o -exec chmod +x {} +").is_allowed());
}

#[test]
fn a_walk_that_follows_links_is_judged_where_they_lead() {
    let p = project(&["src", ".git/hooks", "work/x", "onlylinks"]);
    std::os::unix::fs::symlink("/etc", format!("{}/work/el", p.root)).expect("link out");
    std::os::unix::fs::symlink(format!("{}/.git", p.root), format!("{}/work/gl", p.root)).expect("link in");
    std::os::unix::fs::symlink("../.git/hooks", format!("{}/onlylinks/hk", p.root)).expect("link to hooks");
    for cmd in [
        "find -L work -exec chmod +x {} +",
        "find work -follow -exec chmod +x {} +",
        "fd -L . work -x chmod +x",
        "fd --follow -H . work -x chmod +x",
        "cp -RL work y",
        "rsync -aL work/ y",
        "rsync -a --copy-links work y",
        "find -L work -print0 | xargs -0 chmod +x",
        "find work -exec chmod +x {} +",
        "fd . work -x chmod +x",
        "cp -RL onlylinks y",
        "find onlylinks -exec cp src/a.txt {}/pre-commit \\;",
        "fd . onlylinks -x cp src/a.txt {}/pre-commit",
        "ls onlylinks | xargs -I{} cp src/a.txt onlylinks/{}/pre-commit",
    ] {
        assert!(!at(&p, cmd).is_allowed(), "{cmd} was approved");
    }
    for cmd in ["find work -type f -exec chmod +x {} +", "find work -exec rm {} +", "chmod -R 755 work", "cp -R work y", "rsync -a work/ y"]
    {
        assert!(at(&p, cmd).is_allowed(), "{cmd} was refused");
    }
}

fn with_secret_link(link: bool) -> Project {
    let project = project(&["src"]);
    if link {
        std::os::unix::fs::symlink("/etc/ssh/not-a-real-key", format!("{}/k", project.root)).expect("symlink to a fake secret");
    }
    project
}

#[test]
fn tar_follows_a_link_from_the_directory_in_effect() {
    let secret = "/etc/ssh/not-a-real-key";
    let linked = project(&["src", "sub/deep"]);
    std::os::unix::fs::symlink(secret, format!("{}/sub/deep/s", linked.root)).expect("link under the member");
    let plain = project(&["src", "sub/deep"]);
    for cmd in ["tar -chf - -C sub deep", "tar -chf - --directory=sub deep", "tar -h -c -f - -C sub deep", "tar -chf - -C sub -C deep ."] {
        assert!(!at(&linked, cmd).is_allowed(), "{cmd} was approved over a link to a secret");
        assert!(at(&plain, cmd).is_allowed(), "{cmd} was refused in a tree with no such link");
    }

    let outside = tempfile::tempdir_in("/tmp").expect("directory outside the project");
    let abs = outside.path().canonicalize().expect("canonical directory").to_str().expect("utf-8").to_string();
    let beside = format!("tar -chf - -C {} .", climb_to(&plain, std::path::Path::new(&abs)));
    let absolute = format!("tar -chf - -C {abs} .");
    assert!(at(&plain, &beside).is_allowed(), "{beside} was refused with nothing linked there");
    assert!(at(&plain, &absolute).is_allowed(), "an absolute directory with no link was refused");
    std::os::unix::fs::symlink(secret, outside.path().join("s")).expect("link in the other directory");
    assert!(!at(&plain, &beside).is_allowed(), "{beside} was approved over a link to a secret");
    assert!(!at(&plain, &absolute).is_allowed(), "an absolute directory was approved over a link to a secret");
    let replaced = format!("tar -chf - -C sub -C {abs} .");
    assert!(!at(&plain, &replaced).is_allowed(), "a later absolute directory was ignored");
    std::fs::remove_file(outside.path().join("s")).expect("remove the link");
    assert!(at(&plain, &replaced).is_allowed(), "a later absolute directory with no link was refused");
}

#[test]
fn zip_follows_links_in_names_read_from_stdin() {
    let linked = with_secret_link(true);
    let plain = with_secret_link(false);
    for cmd in ["git ls-files | zip -@ -", "git ls-files | zip -@ out.zip", "git ls-files | zip out.zip -@"] {
        assert!(!at(&linked, cmd).is_allowed(), "{cmd} was approved over a link to a secret");
        assert!(at(&plain, cmd).is_allowed(), "{cmd} was refused in a tree with no such link");
    }
}

#[test]
fn ag_follows_links_into_the_path_after_its_pattern() {
    let secret = "/etc/ssh/not-a-real-key";
    let plain = project(&["src"]);
    let scratch = tempfile::tempdir_in("/tmp").expect("folder outside the project");
    let parent = scratch.path().canonicalize().expect("canonical folder");
    let other = parent.join("other");
    let nested = parent.join("p2").join("sub");
    std::fs::create_dir_all(&other).expect("sibling directory");
    std::fs::create_dir_all(&nested).expect("nested sibling");
    let abs = tempfile::tempdir_in("/tmp").expect("absolute directory");
    let abs_path = abs.path().canonicalize().expect("canonical directory");
    let absolute = format!("ag -f x {}", abs_path.display());
    let (to_other, to_nested) = (format!("ag -f x {}", climb_to(&plain, &other)), format!("ag -f x {}", climb_to(&plain, &nested)));
    for cmd in [to_other.as_str(), to_nested.as_str(), absolute.as_str()] {
        assert!(at(&plain, cmd).is_allowed(), "{cmd} was refused with no link");
    }
    std::os::unix::fs::symlink(secret, other.join("s")).expect("link");
    std::os::unix::fs::symlink(secret, nested.join("s")).expect("link");
    std::os::unix::fs::symlink(secret, abs_path.join("s")).expect("link");
    for cmd in [to_other.as_str(), to_nested.as_str(), absolute.as_str()] {
        assert!(!at(&plain, cmd).is_allowed(), "{cmd} was approved over a link to a secret");
    }

    let linked = with_secret_link(true);
    let clean = with_secret_link(false);
    for cmd in ["ag -f --depth 9 x", "ag -f --ignore foo x", "ag --follow --depth 9 x"] {
        assert!(!at(&linked, cmd).is_allowed(), "{cmd} was approved over a link to a secret");
        assert!(at(&clean, cmd).is_allowed(), "{cmd} was refused in a tree with no such link");
    }
}

#[test]
fn ripgrep_follows_links_when_a_valued_option_precedes_the_pattern() {
    let linked = with_secret_link(true);
    let plain = with_secret_link(false);
    for cmd in [
        "rg -L --max-depth 9 x", "rg -L -E utf8 x", "rg -L --iglob '*' x", "rg -L --sort path x", "rg --follow --sort path x",
        "rg -L --encoding utf8 x", "rg -L --color never x", "rg -L --colors 'match:fg:red' x", "rg -L --ignore-file .ignore x",
        "rg -L --path-separator / x", "rg -L --engine default x", "rg -L --max-columns 10 x",
    ] {
        assert!(!at(&linked, cmd).is_allowed(), "{cmd} was approved over a link to a secret");
        assert!(at(&plain, cmd).is_allowed(), "{cmd} was refused in a tree with no such link");
    }
}

#[test]
fn ripgrep_follows_links_when_its_config_path_is_set() {
    let linked = with_secret_link(true);
    let plain = with_secret_link(false);
    for cmd in [
        "RIPGREP_CONFIG_PATH=./rgrc rg x", "RIPGREP_CONFIG_PATH=rgrc rg x", "env RIPGREP_CONFIG_PATH=./rgrc rg x",
        "env RIPGREP_CONFIG_PATH=rgrc rg x",
    ] {
        assert!(!at(&linked, cmd).is_allowed(), "{cmd} was approved over a link to a secret");
        assert!(at(&plain, cmd).is_allowed(), "{cmd} was refused in a tree with no such link");
    }
    for cmd in [
        "rg x",
        "RIPGREP_CONFIG_PATH= rg x",
        "RIPGREP_CONFIG_PATH=./rgrc rg --no-config x",
        "env RIPGREP_CONFIG_PATH=./rgrc rg --no-config x",
    ] {
        assert!(at(&linked, cmd).is_allowed(), "{cmd} was refused");
    }
}

#[test]
fn pax_7z_ack_and_diff_follow_links_in_the_tree_they_read() {
    let secret = "/etc/ssh/not-a-real-key";
    let linked = project(&["src", "sub", "empty"]);
    std::os::unix::fs::symlink(secret, format!("{}/sub/s", linked.root)).expect("link");
    let plain = project(&["src", "sub", "empty"]);
    for cmd in [
        "pax -wL sub", "pax -w -L sub", "pax -w -H sub", "git ls-files | pax -wL", "7z a -l out.7z sub", "7z a out.7z sub",
        "ack --follow x", "diff -rN sub empty", "diff -ruN empty sub",
    ] {
        assert!(!at(&linked, cmd).is_allowed(), "{cmd} was approved over a link to a secret");
        assert!(at(&plain, cmd).is_allowed(), "{cmd} was refused in a tree with no such link");
    }
    for cmd in [
        "pax -w sub", "pax -w -L -P sub", "7z a -snl out.7z sub", "ack x", "ack --follow --nofollow x", "diff sub empty",
        "diff -rN --no-dereference sub empty",
    ] {
        assert!(at(&linked, cmd).is_allowed(), "{cmd} was refused");
    }
}

#[test]
fn a_command_that_follows_links_is_refused_when_one_leads_to_a_secret() {
    let linked = with_secret_link(true);
    let plain = with_secret_link(false);
    for cmd in [
        "git ls-files | cpio -oL", "tar -chf - .", "tar -cf - --dereference .", "rg -L x", "rg --follow x", "ag -f x", "zip -r - .",
        "zip -0r - .",
    ] {
        assert!(!at(&linked, cmd).is_allowed(), "{cmd} was approved over a link to a secret");
        assert!(at(&plain, cmd).is_allowed(), "{cmd} was refused in a project with no such link");
    }
    for cmd in ["git ls-files | cpio -o", "tar -cf - .", "rg x", "ag x", "zip -y -r - .", "zip -yr - ."] {
        assert!(at(&linked, cmd).is_allowed(), "{cmd} was refused though it does not follow the link");
    }
}

#[test]
fn a_link_to_an_ordinary_outside_file_is_read() {
    let linked = project(&["src"]);
    let link = format!("{}/k", linked.root);
    std::os::unix::fs::symlink("/etc/hosts", &link).expect("link to an ordinary file");
    for cmd in ["zip -r out.zip .", "7z a out.7z .", "tar -chf - ."] {
        assert!(at(&linked, cmd).is_allowed(), "{cmd} was refused over a link to an ordinary file");
    }
    std::fs::remove_file(&link).expect("remove the ordinary link");
    std::os::unix::fs::symlink("/etc/ssh/not-a-real-key", &link).expect("link to a secret");
    for cmd in ["zip -r out.zip .", "7z a out.7z .", "tar -chf - ."] {
        assert!(!at(&linked, cmd).is_allowed(), "{cmd} was approved over a link to a secret");
    }
}

#[test]
fn a_read_that_follows_links_is_judged_where_they_lead() {
    let p = project(&["src", "work/x"]);
    let key = "/etc/ssh/ssh_host_ed25519_key";
    std::os::unix::fs::symlink(key, format!("{}/work/k", p.root)).expect("link to a key");
    assert!(!at(&p, &format!("cat {key}")).is_allowed());
    for cmd in ["find -L work -exec cat {} +", "fd -L . work -x cat", "cp -RL work y", "find work -exec cat {} +", "fd . work -x cat"] {
        assert!(!at(&p, cmd).is_allowed(), "{cmd} was approved");
    }
    for cmd in ["find work -type f -exec cat {} +", "cp -R work y"] {
        assert!(at(&p, cmd).is_allowed(), "{cmd} was refused");
    }
}

#[test]
fn a_followed_link_within_the_project_is_walked_not_refused() {
    let p = project(&["src", "work/x", ".git/hooks"]);
    std::os::unix::fs::symlink("../src", format!("{}/work/in", p.root)).expect("link in the project");
    assert!(at(&p, "find -L work -exec chmod +x {} +").is_allowed());
    assert!(at(&p, "cp -RL work y").is_allowed());
}

#[test]
fn rsync_keeping_destination_links_is_judged_through_them() {
    let p = project(&["src", "d"]);
    std::os::unix::fs::symlink("/etc", format!("{}/d/el", p.root)).expect("link in the destination");
    assert!(!at(&p, "rsync -aK src/ d/").is_allowed());
    assert!(!at(&p, "rsync -a --keep-dirlinks src/ d/").is_allowed());
    assert!(at(&p, "rsync -a src/ d/").is_allowed());
}

#[test]
fn a_prune_find_does_not_test_above_its_minimum_depth_keeps_nothing_away() {
    let p = project(&["src", ".git/hooks", "sub/.git/hooks"]);
    for cmd in [
        "find . -mindepth 2 -name .git -prune -o -exec chmod +x {} +",
        "find . -mindepth 2 \\( -path ./.git -o -path ./sub/.git \\) -prune -o -exec chmod +x {} +",
        "find . -mindepth 2 -type d -name .git -prune -o -type f -exec chmod +x {} +",
        "find . -name .git -prune -o -mindepth 2 -exec chmod +x {} +",
    ] {
        assert!(!at(&p, cmd).is_allowed(), "{cmd} was approved");
    }
}

#[test]
fn a_pax_copy_is_judged_by_what_it_writes_below_its_destination() {
    let p = project(&["src", ".git/hooks", "other/.git/hooks", "hooks", "dest"]);
    for cmd in [
        "pax -rw other dest", "pax -r -w other dest", "pax -wr other dest", "pax -rw hooks .git", "pax -rw src .", "pax -rw src .jj",
        "pax -rwk src .git/hooks", "pax -rw -s ,^,../../, src dest",
    ] {
        assert!(!at(&p, cmd).is_allowed(), "{cmd} was approved");
    }
    for cmd in ["pax -rw src dest", "pax -r -w -p e src dest", "pax -w -f out.tar src", "pax -f out.tar"] {
        assert!(at(&p, cmd).is_allowed(), "{cmd} was refused");
    }
}

#[test]
fn every_spelling_of_a_pax_copy_into_frozen_metadata_is_refused() {
    let p = project(&["src", "dest"]);
    for mode in ["-rw", "-wr", "-r -w", "-w -r", "-rwv", "-v -r -k -w", "-rwpe"] {
        for frozen in [".git", ".jj", ".hg", ".sl", ".envrc", "sub/.git/hooks", "dest/.jj/repo"] {
            let cmd = format!("pax {mode} src {frozen}");
            assert!(!at(&p, &cmd).is_allowed(), "{cmd} was approved");
        }
        let cmd = format!("pax {mode} src dest");
        assert!(at(&p, &cmd).is_allowed(), "{cmd} was refused");
    }
}

#[test]
fn zip_reading_names_from_a_pipe_is_refused_when_the_source_of_names_is_unknown() {
    let plain = with_secret_link(false);
    for cmd in ["cat names.txt | zip -@ out.zip", "cat names.txt | zip -y -@ out.zip", "cat names.txt | zip -y --names-stdin out.zip"] {
        assert!(!at(&plain, cmd).is_allowed(), "{cmd} was approved with names nothing can check");
    }
}

#[test]
fn zip_after_the_option_terminator_reads_its_operands_but_not_the_terminator() {
    let linked = with_secret_link(true);
    for cmd in ["zip -r out.zip -- k", "zip -r -- out.zip k"] {
        assert!(!at(&linked, cmd).is_allowed(), "{cmd} was approved over a link to a secret");
    }
    // A file named `--` is no operand: the terminator itself is not read as an input.
    std::os::unix::fs::symlink("/etc/ssh/not-a-real-key", format!("{}/--", linked.root)).expect("link named as the terminator");
    assert!(at(&linked, "zip -r out.zip -- src").is_allowed(), "the option terminator was read as an input");
}
