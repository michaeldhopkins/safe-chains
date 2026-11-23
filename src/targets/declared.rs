use super::UNKNOWN_WORKDIR;

/// The directory a command runs in when the call can name one (`Cwd`, `dir_path`, `directory`)
/// beside the cwd the harness reports, and whether that directory replaces the reported one.
/// A call that names none runs in the reported folder. One that names an absolute folder runs
/// there; the bool is true when that folder is not the one the harness reported, so a model-chosen
/// directory never becomes the project root. Anything else (a relative or empty name) is a
/// directory the hook cannot place, so the command is judged in the unknown folder.
pub(crate) fn declared_cwd(reported: Option<String>, declared: Option<&str>) -> (Option<String>, bool) {
    match declared {
        None => (reported, false),
        Some(dir) if dir.starts_with('/') => (Some(dir.to_string()), reported.as_deref() != Some(dir)),
        Some(_) => (Some(UNKNOWN_WORKDIR.to_string()), false),
    }
}

/// The project while a command runs in `cwd`. A directory the model named that is not inside the
/// project is where the writes and the code land, so the project stops being the root. A named
/// directory inside the project keeps it. The directory the harness reported keeps it too.
pub(crate) fn project_root_beside(cwd_declared: bool, cwd: &str, project: Option<String>) -> String {
    let placed = placed_directory(cwd);
    // Open the root the same way. A project typed through a link still holds a directory that
    // resolves inside it; comparing the typed spelling drops that project.
    let inside = project
        .as_deref()
        .is_some_and(|root| directory_holds(&placed, &crate::pathctx::physical_directory(root)));
    if cwd_declared && !inside { UNKNOWN_WORKDIR.to_string() } else { project.unwrap_or_else(|| cwd.to_string()) }
}

/// The directory `chdir(2)` opens for `cwd`. Folding `..` before the link is followed climbs
/// from the link, and the kernel climbs from the link's target.
fn placed_directory(cwd: &str) -> String {
    crate::pathctx::join_cwd_chdir(None, cwd).unwrap_or_else(|| cwd.to_string())
}

fn directory_holds(dir: &str, root: &str) -> bool {
    let root = root.trim_end_matches('/');
    let dir = dir.trim_end_matches('/');
    !root.is_empty() && (dir == root || dir.starts_with(&format!("{root}/")))
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest::proptest! {
        #[test]
        fn a_named_directory_is_used_only_when_absolute_and_otherwise_unknown(dir in ".{0,24}", reported in "/[a-z]{1,8}") {
            let (got, declared_it) = declared_cwd(Some(reported.clone()), Some(&dir));
            let absolute = dir.starts_with('/');
            let expected = if absolute { dir.clone() } else { UNKNOWN_WORKDIR.to_string() };
            proptest::prop_assert_eq!(got, Some(expected));
            proptest::prop_assert_eq!(declared_it, absolute && dir != reported);
            proptest::prop_assert_eq!(declared_cwd(Some(reported.clone()), None), (Some(reported), false));
        }
    }

    #[test]
    fn a_named_directory_outside_the_project_drops_the_project_root() {
        let project = Some("/projects/app".to_string());
        let unknown = UNKNOWN_WORKDIR.to_string();
        assert_eq!(project_root_beside(true, "/projects/other", project.clone()), unknown);
        assert_eq!(project_root_beside(true, "/projects/app-extra", project.clone()), unknown);
        assert_eq!(project_root_beside(true, "/projects/app", project.clone()), "/projects/app");
        assert_eq!(project_root_beside(true, "/projects/app/", project.clone()), "/projects/app");
        assert_eq!(project_root_beside(true, "/projects/app/src", project.clone()), "/projects/app");
        assert_eq!(project_root_beside(true, "/projects/app/src", Some("/projects/app/".to_string())), "/projects/app/");
        assert_eq!(project_root_beside(false, "/projects/other", project.clone()), "/projects/app");
        assert_eq!(project_root_beside(true, "/projects/other", None), unknown);
        assert_eq!(project_root_beside(false, "/projects/other", None), "/projects/other");
    }

    #[test]
    fn a_named_directory_that_climbs_out_of_the_project_drops_the_project_root() {
        let project = Some("/projects/app".to_string());
        let unknown = UNKNOWN_WORKDIR.to_string();
        assert_eq!(project_root_beside(true, "/projects/app/../other", project.clone()), unknown);
        assert_eq!(project_root_beside(true, "/projects/app/src/../lib", project), "/projects/app");
    }

    #[test]
    fn a_named_link_to_another_workspace_drops_the_project_root() {
        let base = tempfile::tempdir().expect("tempdir");
        let app = base.path().join("app");
        let other = base.path().join("other");
        std::fs::create_dir(&app).expect("app");
        std::fs::create_dir(&other).expect("other");
        std::fs::create_dir(other.join("sub")).expect("sub");
        std::os::unix::fs::symlink("../other/sub", app.join("lnk")).expect("link");
        let app_text = app.to_str().expect("utf-8 app");
        let link_text = app.join("lnk").to_str().expect("utf-8 link").to_string();
        assert_eq!(project_root_beside(true, &link_text, Some(app_text.to_string())), UNKNOWN_WORKDIR.to_string());
        // `..` climbs from the link's target (`other`), not from the link's parent (`app`).
        let climbed = app.join("lnk").join("..");
        let climbed_text = climbed.to_str().expect("utf-8 climbed");
        assert_eq!(project_root_beside(true, climbed_text, Some(app_text.to_string())), UNKNOWN_WORKDIR.to_string());
    }

    #[test]
    fn a_project_reached_through_a_link_keeps_its_root() {
        let base = tempfile::tempdir().expect("tempdir");
        let app = base.path().join("real").join("app");
        std::fs::create_dir_all(app.join("src")).expect("app");
        std::os::unix::fs::symlink("real", base.path().join("lnk")).expect("link");
        let through = base.path().join("lnk").join("app");
        let through_text = through.to_str().expect("utf-8 project").to_string();
        let src_text = through.join("src").to_str().expect("utf-8 src").to_string();
        assert_eq!(project_root_beside(true, &src_text, Some(through_text.clone())), through_text);
    }
}
