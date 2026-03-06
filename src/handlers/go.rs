use crate::parse::{Segment, Token, WordSet};
use crate::policy::{self, FlagPolicy, FlagStyle};

static GO_VERSION_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["-m", "-v"]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GO_ENV_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["-json"]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GO_DOC_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["-all", "-c", "-cmd", "-short", "-src", "-u"]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GO_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "-a", "-asan", "-compiled", "-cover", "-deps", "-e", "-export",
        "-find", "-linkshared", "-m", "-modcacherw", "-msan", "-n",
        "-race", "-retract", "-test", "-trimpath", "-u", "-v",
        "-versions", "-work", "-x",
    ]),
    standalone_short: b"",
    valued: WordSet::new(&[
        "-asmflags", "-buildmode", "-buildvcs", "-compiler", "-covermode",
        "-coverpkg", "-f", "-gccgoflags", "-gcflags", "-installsuffix",
        "-json", "-ldflags", "-mod", "-modfile", "-overlay", "-p",
        "-pgo", "-pkgdir", "-reuse", "-tags",
    ]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GO_VET_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "-a", "-asan", "-cover", "-json", "-linkshared", "-modcacherw",
        "-msan", "-n", "-race", "-trimpath", "-v", "-work", "-x",
    ]),
    standalone_short: b"",
    valued: WordSet::new(&[
        "-asmflags", "-buildmode", "-buildvcs", "-c", "-compiler",
        "-covermode", "-coverpkg", "-gccgoflags", "-gcflags",
        "-installsuffix", "-ldflags", "-mod", "-modfile", "-overlay",
        "-p", "-pgo", "-pkgdir", "-tags",
    ]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GO_BUILD_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "-a", "-asan", "-cover", "-linkshared", "-modcacherw",
        "-msan", "-n", "-race", "-trimpath", "-v", "-work", "-x",
    ]),
    standalone_short: b"",
    valued: WordSet::new(&[
        "-asmflags", "-buildmode", "-buildvcs", "-compiler", "-covermode",
        "-coverpkg", "-gccgoflags", "-gcflags", "-installsuffix",
        "-ldflags", "-mod", "-modfile", "-o", "-overlay", "-p",
        "-pgo", "-pkgdir", "-tags",
    ]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GO_TEST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "-a", "-asan", "-benchmem", "-cover", "-failfast", "-json",
        "-linkshared", "-modcacherw", "-msan", "-n", "-race",
        "-short", "-trimpath", "-v", "-work", "-x",
    ]),
    standalone_short: b"",
    valued: WordSet::new(&[
        "-asmflags", "-bench", "-benchtime", "-blockprofile",
        "-blockprofilerate", "-buildmode", "-buildvcs", "-compiler",
        "-count", "-covermode", "-coverpkg", "-coverprofile",
        "-cpu", "-cpuprofile", "-fuzz", "-fuzzminimizetime", "-fuzztime",
        "-gccgoflags", "-gcflags", "-installsuffix",
        "-ldflags", "-list", "-memprofile", "-memprofilerate",
        "-mod", "-modfile", "-mutexprofile", "-mutexprofilefraction",
        "-o", "-outputdir", "-overlay", "-p", "-parallel",
        "-pgo", "-pkgdir", "-run", "-shuffle", "-skip",
        "-tags", "-timeout", "-trace",
    ]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub fn is_safe_go(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    let policy = match tokens[1].as_str() {
        "help" => return true,
        "build" => &GO_BUILD_POLICY,
        "doc" => &GO_DOC_POLICY,
        "env" => &GO_ENV_POLICY,
        "list" => &GO_LIST_POLICY,
        "test" => &GO_TEST_POLICY,
        "version" => &GO_VERSION_POLICY,
        "vet" => &GO_VET_POLICY,
        _ => return false,
    };
    policy::check(&tokens[1..], policy)
}

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    match cmd {
        "go" => Some(is_safe_go(tokens)),
        _ => None,
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![CommandDoc::handler("go",
        "Subcommands: build, doc, env, help, list, test, version, vet.")]
}

#[cfg(test)]
pub(super) const REGISTRY: &[super::CommandEntry] = &[
    super::CommandEntry::Subcommand { cmd: "go", subs: &[
        super::SubEntry::Policy { name: "build" },
        super::SubEntry::Policy { name: "doc" },
        super::SubEntry::Policy { name: "env" },
        super::SubEntry::Positional { name: "help" },
        super::SubEntry::Policy { name: "list" },
        super::SubEntry::Policy { name: "test" },
        super::SubEntry::Policy { name: "version" },
        super::SubEntry::Policy { name: "vet" },
    ]},
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        go_help_bare: "go help",
        go_help_build: "go help build",
        go_help_modules: "go help modules",
        go_help_test: "go help test",
        go_version: "go version",
        go_version_flag: "go --version",
        go_version_m: "go version -m /usr/local/go/bin/go",
        go_env: "go env GOPATH",
        go_env_bare: "go env",
        go_env_json: "go env -json",
        go_env_json_var: "go env -json GOPATH",
        go_list: "go list ./...",
        go_list_json: "go list -json ./...",
        go_list_m: "go list -m all",
        go_list_f: "go list -f '{{.Dir}}' ./...",
        go_list_deps: "go list -deps ./...",
        go_list_tags: "go list -tags=integration ./...",
        go_vet: "go vet ./...",
        go_vet_json: "go vet -json ./...",
        go_vet_tags: "go vet -tags integration ./...",
        go_test: "go test ./...",
        go_test_verbose: "go test -v ./...",
        go_test_run: "go test -run TestFoo ./...",
        go_test_count: "go test -count=1 ./...",
        go_test_race: "go test -race -v -count=1 ./...",
        go_test_short: "go test -short ./...",
        go_test_timeout: "go test -timeout 30s ./...",
        go_test_cover: "go test -coverprofile=coverage.out ./...",
        go_test_bench: "go test -bench . -benchmem ./...",
        go_test_shuffle: "go test -shuffle=on ./...",
        go_build: "go build ./...",
        go_build_race: "go build -race ./...",
        go_build_tags: "go build -tags=integration ./...",
        go_build_verbose: "go build -v -x ./...",
        go_build_ldflags: "go build -ldflags '-s -w' ./...",
        go_build_trimpath: "go build -trimpath ./...",
        go_build_output: "go build -o myapp ./cmd/app",
        go_doc: "go doc fmt.Println",
        go_doc_all: "go doc -all fmt",
        go_doc_src: "go doc -src fmt.Println",
    }

    denied! {
        bare_go_denied: "go",
        go_build_toolexec_denied: "go build -toolexec=cmd ./...",
        go_build_toolexec_space_denied: "go build -toolexec cmd ./...",
        go_test_exec_denied: "go test -exec=cmd ./...",
        go_test_exec_space_denied: "go test -exec cmd ./...",
        go_test_toolexec_denied: "go test -toolexec=cmd ./...",
        go_env_w_denied: "go env -w GOPATH=/tmp",
        go_env_u_denied: "go env -u GOPATH",
        go_vet_vettool_denied: "go vet -vettool=mytool ./...",
        go_list_toolexec_denied: "go list -toolexec=cmd ./...",
    }
}
