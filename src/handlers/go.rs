use crate::command::{CommandDef, SubDef};
use crate::verdict::{SafetyLevel, Verdict};
use crate::parse::{Token, WordSet};
use crate::policy::{FlagPolicy, FlagStyle};

static GO_VERSION_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h", "-m", "-v"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GO_ENV_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h", "-json"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GO_DOC_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-all", "-c", "-cmd", "-h", "-short", "-src", "-u"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GO_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--help",
        "-a", "-asan", "-compiled", "-cover", "-deps", "-e", "-export",
        "-find", "-h", "-linkshared", "-m", "-modcacherw", "-msan", "-n",
        "-race", "-retract", "-test", "-trimpath", "-u", "-v",
        "-versions", "-work", "-x",
    ]),
    valued: WordSet::flags(&[
        "-asmflags", "-buildmode", "-buildvcs", "-compiler", "-covermode",
        "-coverpkg", "-f", "-gccgoflags", "-gcflags", "-installsuffix",
        "-json", "-ldflags", "-mod", "-modfile", "-overlay", "-p",
        "-pgo", "-pkgdir", "-reuse", "-tags",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GO_VET_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--help",
        "-a", "-asan", "-cover", "-h", "-json", "-linkshared", "-modcacherw",
        "-msan", "-n", "-race", "-trimpath", "-v", "-work", "-x",
    ]),
    valued: WordSet::flags(&[
        "-asmflags", "-buildmode", "-buildvcs", "-c", "-compiler",
        "-covermode", "-coverpkg", "-gccgoflags", "-gcflags",
        "-installsuffix", "-ldflags", "-mod", "-modfile", "-overlay",
        "-p", "-pgo", "-pkgdir", "-tags",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GO_BUILD_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--help",
        "-a", "-asan", "-cover", "-h", "-linkshared", "-modcacherw",
        "-msan", "-n", "-race", "-trimpath", "-v", "-work", "-x",
    ]),
    valued: WordSet::flags(&[
        "-asmflags", "-buildmode", "-buildvcs", "-compiler", "-covermode",
        "-coverpkg", "-gccgoflags", "-gcflags", "-installsuffix",
        "-ldflags", "-mod", "-modfile", "-o", "-overlay", "-p",
        "-pgo", "-pkgdir", "-tags",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GO_TEST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--help",
        "-a", "-asan", "-benchmem", "-cover", "-failfast", "-h", "-json",
        "-linkshared", "-modcacherw", "-msan", "-n", "-race",
        "-short", "-trimpath", "-v", "-work", "-x",
    ]),
    valued: WordSet::flags(&[
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
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

fn check_go_help(_tokens: &[Token]) -> Verdict {
    Verdict::Allowed(SafetyLevel::Inert)

}

pub(crate) static GO: CommandDef = CommandDef {
    name: "go",
    subs: &[
        SubDef::Policy { name: "build", policy: &GO_BUILD_POLICY, level: SafetyLevel::SafeWrite },
        SubDef::Policy { name: "doc", policy: &GO_DOC_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "env", policy: &GO_ENV_POLICY, level: SafetyLevel::Inert },
        SubDef::Custom { name: "help", check: check_go_help, doc: "", test_suffix: None },
        SubDef::Policy { name: "list", policy: &GO_LIST_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "test", policy: &GO_TEST_POLICY, level: SafetyLevel::SafeRead },
        SubDef::Policy { name: "version", policy: &GO_VERSION_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "vet", policy: &GO_VET_POLICY, level: SafetyLevel::SafeRead },
    ],
    bare_flags: &["--help", "--version", "-V", "-h"],
    url: "https://pkg.go.dev/cmd/go",
    aliases: &[],
};

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    GO.dispatch(cmd, tokens)
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![GO.to_doc()]
}

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
