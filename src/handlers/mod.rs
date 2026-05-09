macro_rules! handler_module {
    ($($sub:ident),+ $(,)?) => {
        $(mod $sub;)+

        pub(crate) fn dispatch(cmd: &str, tokens: &[crate::parse::Token]) -> Option<crate::verdict::Verdict> {
            None$(.or_else(|| $sub::dispatch(cmd, tokens)))+
        }

        pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
            let mut docs = Vec::new();
            $(docs.extend($sub::command_docs());)+
            docs
        }

        #[cfg(test)]
        pub(super) fn full_registry() -> Vec<&'static super::CommandEntry> {
            let mut v = Vec::new();
            $(v.extend($sub::REGISTRY);)+
            v
        }
    };
}

pub mod android;
pub mod coreutils;
pub mod forges;
pub mod fuzzy;
pub mod jvm;
pub mod magick;
pub mod network;
pub mod node;
pub mod perl;
pub mod php;
pub mod ruby;
pub mod shell;
pub mod system;
pub mod vcs;
pub mod wrappers;

use std::collections::HashMap;

use crate::parse::Token;
use crate::verdict::Verdict;

type HandlerFn = fn(&[Token]) -> Verdict;

pub fn custom_cmd_handlers() -> HashMap<&'static str, HandlerFn> {
    HashMap::from([
        ("magick", magick::is_safe_magick as HandlerFn),
        ("php", php::is_safe_php as HandlerFn),
        ("sysctl", system::sysctl::is_safe_sysctl as HandlerFn),
    ])
}

pub fn custom_sub_handlers() -> HashMap<&'static str, HandlerFn> {
    HashMap::from([
        ("bun_x", node::bun::check_bun_x as HandlerFn),
        ("bundle_config", ruby::bundle::check_bundle_config as HandlerFn),
        ("bundle_exec", ruby::bundle::check_bundle_exec as HandlerFn),
        ("git_remote", vcs::git::check_git_remote as HandlerFn),
        ("laravel_cache_clear", php::check_laravel_cache_clear as HandlerFn),
        ("plutil_convert", system::plutil::check_plutil_convert as HandlerFn),
    ])
}

pub fn dispatch(tokens: &[Token]) -> Verdict {
    let cmd = tokens[0].command_name();
    None
        .or_else(|| shell::dispatch(cmd, tokens))
        .or_else(|| wrappers::dispatch(cmd, tokens))
        .or_else(|| forges::dispatch(cmd, tokens))
        .or_else(|| node::dispatch(cmd, tokens))
        .or_else(|| jvm::dispatch(cmd, tokens))
        .or_else(|| android::dispatch(cmd, tokens))
        .or_else(|| network::dispatch(cmd, tokens))
        .or_else(|| system::dispatch(cmd, tokens))
        .or_else(|| perl::dispatch(cmd, tokens))
        .or_else(|| coreutils::dispatch(cmd, tokens))
        .or_else(|| fuzzy::dispatch(cmd, tokens))
        .or_else(|| vcs::dispatch(cmd, tokens))
        .or_else(|| crate::registry::toml_dispatch(tokens))
        .unwrap_or(Verdict::Denied)
}

#[cfg(test)]
const HANDLED_CMDS: &[&str] = &[
    "sh", "bash", "xargs", "timeout", "time", "env", "nice", "ionice", "hyperfine", "dotenv", "jai",
    "git", "jj", "gh", "glab", "jjpr", "tea", "basecamp",
    "jira", "linear", "notion", "td", "todoist", "trello",
    "npm", "yarn", "pnpm", "bun", "deno", "npx", "bunx", "nvm", "fnm", "volta", "mocha",
    "ruby", "ri", "bundle", "gem", "importmap", "rails", "rbenv", "rvm", "brakeman", "rspec",
    "standardrb", "erb_lint", "erblint", "herb",
    "reek", "flay", "flog", "fasterer", "haml-lint", "slim-lint",
    "bundler-audit", "bundle-audit", "ruby-audit", "rdoc", "yard", "yardoc", "rubycritic",
    "annotaterb", "annotate", "jekyll", "bridgetown", "middleman", "foreman", "guard",
    "spring", "overcommit", "pry", "byebug", "thor", "m", "rake", "sdoc", "license_finder",
    "danger", "kamal", "mutant", "whenever", "haml", "slimrb", "railroady", "erd",
    "parallel_test", "parallel_rspec", "parallel_cucumber", "parallel_spinach",
    "racc", "rex",
    "steep", "srb", "rbs", "typeprof", "stree", "rufo", "packwerk", "debride",
    "i18n-tasks", "asciidoctor", "kramdown", "dawn", "fpm", "stackprof",
    "pipx", "pip-compile", "pip-sync", "pre-commit", "sphinx-build", "sphinx-quickstart", "sphinx-apidoc",
    "mkdocs", "twine", "yapf", "autopep8", "autoflake", "pyupgrade", "vulture", "pyflakes",
    "pycodestyle", "pydocstyle", "cookiecutter", "copier", "deptry", "safety",
    "http", "https", "ipython", "scalene", "py-spy", "kernprof", "mprof", "dvc", "alembic", "hatch",
    "husky", "lint-staged", "markdownlint-cli2", "markdownlint", "typedoc", "nodemon", "pm2",
    "ncu", "depcruise", "dependency-cruise",
    "rollup", "vite", "esbuild", "swc", "webpack", "parcel", "tsup",
    "prisma", "drizzle-kit", "sequelize", "knex",
    "ava", "tap", "c8", "nyc", "jasmine",
    "http-server", "serve", "concurrently",
    "npm-run-all", "run-p", "run-s",
    "tsx", "ts-node", "cucumber-js", "@cucumber/cucumber",
    "bacon", "sccache", "sqlx", "diesel", "starship", "atuin",
    "gofmt", "goimports", "gofumpt", "gci", "revive", "errcheck",
    "gotestsum", "goreleaser", "mage", "task", "buf", "gosec", "gomodifytags", "dlv",
    "scala", "scalac", "sbt", "mill", "groovy", "lein", "clj", "clojure",
    "kotlinc", "scalafmt", "scalafix", "jdeps", "jcmd", "jstack",
    "ghc", "cabal", "stack", "hlint", "ormolu", "fourmolu",
    "opam", "dune", "ocamlformat",
    "credo", "iex",
    "clang-format", "clang-tidy", "cppcheck", "doxygen", "autoconf", "automake", "cmake-format",
    "crystal", "shards", "ameba",
    "nim", "nimble",
    "luarocks", "selene",
    "julia",
    "dart", "flutter",
    "hugo", "zola", "eleventy", "@11ty/eleventy", "gatsby", "astro", "vitepress", "hexo",
    "op", "bw", "pass", "vault", "gpg", "age", "sops",
    "terraform-docs", "tflint", "tfsec", "terragrunt", "ansible-lint", "helmfile",
    "argocd", "skaffold", "tilt", "consul", "nomad",
    "jupyter", "jupytext", "nbqa", "jupyter-nbconvert", "nbconvert", "nbstripout",
    "mlflow", "wandb", "papermill", "dbt",
    "rebar3", "fantomas", "cpan", "cpanm", "plenv", "carton",
    "latexmk", "pdflatex", "xelatex", "lualatex", "latex", "biber",
    "dub", "sbcl", "ros", "raco", "gleam", "roc",
    "ffmpeg", "ffprobe", "exiftool", "mediainfo",
    "jpegoptim", "optipng", "pngquant", "gifsicle", "cwebp", "sox",
    "pandoc", "marp",
    "rsync", "rclone", "restic", "borg", "mc",
    "mysql", "mariadb", "mongosh", "redis-cli", "sqlite3", "duckdb", "usql", "pg_restore",
    "entr", "parallel", "ts", "sponge", "vipe", "vidir", "chronic", "ifne", "errno", "isutf8", "pee",
    "buildah", "velero", "flux", "linkerd", "istioctl", "kapp", "ytt",
    "nu", "pscale", "supabase", "neon", "neonctl",
    "nuget", "paket", "msbuild", "cake", "nuke", "docfx",
    "mono", "mcs", "fsi", "dotnet-fsi", "csi",
    "dotnet-ef", "dotnet-script", "dotnet-counters", "dotnet-dump",
    "dotnet-trace", "dotnet-stack", "dotnet-gcdump", "dotnet-sos", "dotnet-symbol",
    "7z", "7zz", "7za", "zstd", "unzstd", "zstdcat", "zstdmt", "xz", "unxz", "xzcat",
    "lzma", "unlzma", "lzcat", "bzip2", "bunzip2", "bzcat", "bzip2recover", "brotli",
    "lz4", "unlz4", "lz4cat", "lz4c", "pigz", "unpigz", "pbzip2", "lzip", "lunzip", "plzip", "ar",
    "openssl", "mkcert", "certbot",
    "ssh-keygen", "ssh-keyscan", "ssh-copy-id", "ssh-add", "ssh-agent",
    "scp", "sftp", "sshfs", "autossh", "mosh",
    "caddy", "nginx", "haproxy", "envoy", "traefik",
    "hadolint", "dive", "nerdctl", "ctr", "copa",
    "wasmtime", "wasmer", "wasm-pack", "wat2wasm", "wasm2wat",
    "wasm-validate", "wasm-objdump", "wasm-strip", "wasm-tools", "spin",
    "hg", "svn", "fossil", "pijul",
    "newman", "bru", "inso", "ab", "wrk", "iperf3", "iperf",
    "vegeta", "k6", "locust", "bombardier", "siege", "artillery",
    "huggingface-cli", "sentry-cli", "promtool", "grafana-cli", "amtool", "otel-cli", "oc",
    "arduino-cli", "pio", "platformio", "esptool", "esptool.py",
    "avrdude", "openocd", "west", "idf.py",
    "coqc", "coqtop", "lean", "lake", "elan", "agda", "idris2", "swipl",
    "code", "code-insiders", "codium", "hx", "subl",
    "earthly", "buck2", "pants", "waf", "xmake",
    "shfmt", "yamllint", "jsonlint", "editorconfig-checker", "ec",
    "pkg-config", "envsubst",
    "ansible-vault", "molecule",
    "nix", "nix-shell", "nix-build", "nix-env", "nix-store", "nix-collect-garbage",
    "nix-channel", "nix-instantiate", "nix-prefetch-url",
    "nixos-rebuild", "home-manager", "devenv",
    "sam", "cdk", "amplify", "eb", "sls", "serverless", "copilot", "chalice",
    "expo", "react-native", "ionic", "cap", "capacitor", "tns", "ns", "nativescript",
    "create-vite", "create-next-app", "create-react-app",
    "degit", "tiged", "yo", "hygen", "plop",
    "pg_dumpall", "pg_basebackup", "pg_ctl", "pg_config", "pg_controldata",
    "createdb", "dropdb", "createuser", "dropuser",
    "vacuumdb", "reindexdb", "clusterdb", "pgbench",
    "mysqldump", "mysqladmin", "mysqlcheck",
    "mongodump", "mongorestore", "mongoimport", "mongoexport", "mongostat", "mongotop",
    "redis-server", "redis-benchmark", "cqlsh", "nodetool", "cockroach", "influx", "influxd",
    "atlas", "flyway", "liquibase", "dbmate", "migrate", "goose",
    "pgcli", "mycli", "litecli", "iredis",
    "wget", "aria2c",
    "gs", "qpdf", "pdftk", "pdftotext", "pdftohtml", "pdftocairo",
    "pdfimages", "pdfinfo", "pdffonts", "pdfdetach", "pdfseparate", "pdfunite",
    "weasyprint", "wkhtmltopdf",
    "emcc", "em++", "emar", "emconfigure", "emmake", "emranlib", "emstrip", "emrun",
    "llc", "opt", "lli", "llvm-cov", "llvm-objdump", "llvm-readobj", "llvm-readelf",
    "llvm-strip", "llvm-mc", "llvm-link", "llvm-as", "llvm-dis",
    "llvm-symbolizer", "llvm-profdata", "llvm-nm", "llvm-ar", "llvm-ranlib",
    "llvm-config", "llvm-cxxfilt",
    "gfortran", "flang", "flang-new", "gnatmake", "gnatls", "gnatchop", "gnatpp",
    "gnatkr", "gnatxref", "gnatfind", "gnatprep", "tcc",
    "godot", "godot4",
    "tclsh", "wish", "expect", "unbuffer",
    "buildctl", "docker-compose", "docker-buildx",
    "chef-client", "chef", "knife", "chef-solo", "chef-shell",
    "puppet", "puppet-agent", "puppet-master",
    "salt", "salt-call", "salt-master", "salt-key", "salt-run", "salt-cloud", "salt-ssh",
    "kn", "tkn", "kpt", "dapr", "oras", "krew",
    "conan", "vcpkg", "spack",
    "tcpdump", "tshark", "arp", "arping", "masscan", "grpcurl", "protoc",
    "ipfs", "xh", "oha", "hurl", "httpyac",
    "lnav", "btop", "glances", "nvtop", "gpustat",
    "pbcopy", "pbpaste", "xclip", "xsel",
    "cast", "forge", "anvil", "chisel", "foundryup",
    "hardhat", "truffle", "solc", "vyper",
    "geth", "solana", "anchor", "spl-token", "near", "near-cli",
    "bitcoin-cli", "lncli", "slither", "mythril", "myth", "ignite", "polkadot",
    "pip", "pip3", "uv", "poetry", "pyenv", "conda", "coverage", "tox", "nox", "bandit", "pip-audit", "pdm",
    "cargo", "rustup",
    "go",
    "gradle", "gradlew", "mvn", "mvnw", "ktlint", "detekt",
    "javap", "jar", "keytool", "jarsigner", "jenv", "sdk",
    "adb", "apkanalyzer", "apksigner", "bundletool", "aapt2",
    "emulator", "avdmanager", "sdkmanager", "zipalign", "lint",
    "fastlane", "firebase",
    "artisan", "composer", "craft", "pest", "phpstan", "phpunit", "please", "valet",
    "swift",
    "dotnet",
    "curl",
    "docker", "podman", "kubectl", "orbctl", "orb", "qemu-img", "helm", "skopeo", "crane", "cosign", "kustomize", "stern", "kubectx", "kubens", "kind", "minikube",
    "ollama", "llm", "hf", "claude", "aider", "codex", "opencode", "vibe",
    "ddev", "dcli",
    "brew", "mise", "asdf", "crontab", "defaults", "pmset", "sysctl", "cmake", "psql", "pg_isready",
    "pg_dump", "bazel", "meson", "ninja",
    "terraform", "heroku", "vercel", "fly", "flyctl", "pulumi", "netlify", "railway", "render",
    "northflank", "porter", "platform", "upsun", "koyeb", "scalingo", "clever",
    "cx", "hey", "wrangler", "cf", "newrelic",
    "aws", "gcloud", "az",
    "doctl", "hcloud", "vultr-cli", "exo", "scw", "linode-cli",
    "ansible-playbook", "ansible-inventory", "ansible-doc", "ansible-config", "ansible-galaxy",
    "overmind", "tailscale", "tmux", "wg", "systemctl", "journalctl", "zellij",
    "kafka-topics", "kafka-console-consumer", "kafka-consumer-groups",
    "monolith",
    "cloudflared", "ngrok", "ssh",
    "networksetup", "launchctl", "diskutil", "security", "csrutil", "log",
    "xcodebuild", "plutil", "xcode-select", "xcrun", "pkgutil", "lipo", "codesign", "spctl",
    "xcodegen", "tuist", "pod", "swiftlint", "swiftformat", "periphery", "xcbeautify", "agvtool", "simctl",
    "perl",
    "R", "Rscript",
    "grep", "egrep", "fgrep", "rg", "ag", "ack", "zgrep", "zegrep", "zfgrep", "locate", "mlocate", "plocate",
    "cat", "gzcat", "head", "tail", "wc", "cut", "tr", "uniq", "less", "more", "zcat",
    "diff", "comm", "paste", "tac", "rev", "nl",
    "expand", "unexpand", "fold", "fmt", "col", "column", "iconv", "nroff",
    "echo", "printf", "seq", "test", "[", "expr", "bc", "factor", "bat", "glow",
    "arch", "command", "hostname",
    "find", "sed", "shuf", "sort", "yq", "xmllint", "awk", "gawk", "mawk", "nawk",
    "magick", "convert", "frames",
    "fd", "eza", "exa", "ls", "delta", "colordiff",
    "dirname", "basename", "realpath", "readlink",
    "file", "stat", "du", "df", "tree", "cmp", "zipinfo", "tar", "unzip", "gzip",
    "true", "false", ":", "shopt",
    "alias", "break", "continue", "declare", "exit", "export", "hash", "printenv", "read", "type", "typeset", "wait", "whereis", "which", "whoami", "date", "pwd", "cd", "unset",
    "uname", "nproc", "uptime", "id", "groups", "tty", "locale", "cal", "sleep",
    "who", "w", "last", "lastlog",
    "ps", "top", "htop", "iotop", "procs", "dust", "lsof", "pgrep", "pstree", "lsblk", "free", "sample", "kill",
    "jq", "jaq", "gojq", "fx", "jless", "htmlq", "xq", "tomlq", "mlr", "dasel",
    "base64", "xxd", "getconf", "uuidgen",
    "md5sum", "md5", "sha256sum", "shasum", "sha1sum", "sha512sum",
    "cksum", "b2sum", "sum", "strings", "hexdump", "od", "size", "sips",
    "sw_vers", "mdls", "otool", "nm", "system_profiler", "ioreg", "vm_stat", "mdfind", "man",
    "dig", "nslookup", "host", "whois", "netstat", "ss", "ifconfig", "route", "ping",
    "traceroute", "traceroute6", "mtr", "nc", "ncat", "nmap",
    "xv",
    "fzf", "fzy", "peco", "pick", "selecta", "sk", "zf",
    "identify", "shellcheck", "cloc", "tokei", "cucumber", "branchdiff", "specdiff", "workon", "safe-chains", "snyk", "mdbook", "devbox", "pup",
    "tldr", "ldd", "objdump", "readelf", "just",
    "prettier", "black", "ruff", "mypy", "pyright", "pylint", "flake8", "isort",
    "rubocop", "eslint", "biome", "stylelint", "zoxide",
    "@herb-tools/linter", "@biomejs/biome", "@commitlint/cli", "@redocly/cli",
    "@axe-core/cli", "@arethetypeswrong/cli", "@taplo/cli", "@johnnymorganz/stylua-bin",
    "@shopify/theme-check", "@graphql-inspector/cli", "@apidevtools/swagger-cli",
    "@astrojs/check", "@changesets/cli",
    "@stoplight/spectral-cli", "@ibm/openapi-validator", "@openapitools/openapi-generator-cli",
    "@ls-lint/ls-lint", "@htmlhint/htmlhint", "@manypkg/cli",
    "@microsoft/api-extractor", "@asyncapi/cli",
    "svelte-check", "secretlint", "oxlint", "knip", "size-limit",
    "depcheck", "madge", "license-checker",
    "pytest", "jest", "vitest", "golangci-lint", "staticcheck", "govulncheck", "semgrep", "next", "turbo", "nx",
    "direnv", "make", "packer", "vagrant",
    "node", "python3", "python", "rustc", "java", "php",
    "gcc", "g++", "cc", "c++", "clang", "clang++",
    "elixir", "erl", "mix", "zig", "lua", "tsc",
    "jc", "gron", "difft", "difftastic", "duf", "xsv", "qsv",
    "git-cliff", "git-lfs", "tig",
    "trivy", "gitleaks", "grype", "syft", "watchexec", "act",
];

pub fn handler_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = Vec::new();
    docs.extend(forges::command_docs());
    docs.extend(node::command_docs());
    docs.extend(jvm::command_docs());
    docs.extend(android::command_docs());
    docs.extend(network::command_docs());
    docs.extend(system::command_docs());
    docs.extend(perl::command_docs());
    docs.extend(coreutils::command_docs());
    docs.extend(fuzzy::command_docs());
    docs.extend(shell::command_docs());
    docs.extend(wrappers::command_docs());
    docs.extend(vcs::command_docs());
    docs.extend(crate::registry::toml_command_docs());
    docs
}

#[cfg(test)]
#[derive(Debug)]
pub(crate) enum CommandEntry {
    Positional { cmd: &'static str },
    Custom { cmd: &'static str, valid_prefix: Option<&'static str> },
    Paths { cmd: &'static str, bare_ok: bool, paths: &'static [&'static str] },
    Delegation { cmd: &'static str },
}

pub fn all_opencode_patterns() -> Vec<String> {
    let mut patterns = Vec::new();
    patterns.sort();
    patterns.dedup();
    patterns
}

#[cfg(test)]
fn full_registry() -> Vec<&'static CommandEntry> {
    let mut entries = Vec::new();
    entries.extend(shell::REGISTRY);
    entries.extend(wrappers::REGISTRY);
    entries.extend(forges::full_registry());
    entries.extend(node::full_registry());
    entries.extend(jvm::full_registry());
    entries.extend(android::full_registry());
    entries.extend(network::REGISTRY);
    entries.extend(system::full_registry());
    entries.extend(perl::REGISTRY);
    entries.extend(coreutils::full_registry());
    entries.extend(fuzzy::full_registry());
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    const UNKNOWN_FLAG: &str = "--xyzzy-unknown-42";
    const UNKNOWN_SUB: &str = "xyzzy-unknown-42";

    fn check_entry(entry: &CommandEntry, failures: &mut Vec<String>) {
        match entry {
            CommandEntry::Positional { .. } | CommandEntry::Delegation { .. } => {}
            CommandEntry::Custom { cmd, valid_prefix } => {
                let base = valid_prefix.unwrap_or(cmd);
                let test = format!("{base} {UNKNOWN_FLAG}");
                if crate::is_safe_command(&test) {
                    failures.push(format!("{cmd}: accepted unknown flag: {test}"));
                }
            }
            CommandEntry::Paths { cmd, bare_ok, paths } => {
                if !bare_ok && crate::is_safe_command(cmd) {
                    failures.push(format!("{cmd}: accepted bare invocation"));
                }
                let test = format!("{cmd} {UNKNOWN_SUB}");
                if crate::is_safe_command(&test) {
                    failures.push(format!("{cmd}: accepted unknown subcommand: {test}"));
                }
                for path in *paths {
                    let test = format!("{path} {UNKNOWN_FLAG}");
                    if crate::is_safe_command(&test) {
                        failures.push(format!("{path}: accepted unknown flag: {test}"));
                    }
                }
            }
        }
    }

    #[test]
    fn all_commands_reject_unknown() {
        let registry = full_registry();
        let mut failures = Vec::new();
        for entry in &registry {
            check_entry(entry, &mut failures);
        }
        assert!(
            failures.is_empty(),
            "unknown flags/subcommands accepted:\n{}",
            failures.join("\n")
        );
    }

    #[test]
    fn process_substitution_safe_inner() {
        let safe = ["echo <(cat /etc/passwd)", "grep pattern <(ls)", "diff <(sort a.txt) <(sort b.txt)", "comm -23 file.txt <(sort other.txt)"];
        for cmd in &safe {
            assert!(crate::is_safe_command(cmd), "safe process substitution rejected: {cmd}");
        }
    }

    #[test]
    fn process_substitution_unsafe_inner() {
        let unsafe_cmds = ["echo >(rm -rf /)", "diff <(sort a.txt) <(rm -rf /)"];
        for cmd in &unsafe_cmds {
            assert!(!crate::is_safe_command(cmd), "unsafe process substitution approved: {cmd}");
        }
    }

    #[test]
    fn registry_covers_handled_commands() {
        let registry = full_registry();
        let mut all_cmds: HashSet<&str> = registry
            .iter()
            .map(|e| match e {
                CommandEntry::Positional { cmd }
                | CommandEntry::Custom { cmd, .. }
                | CommandEntry::Paths { cmd, .. }
                | CommandEntry::Delegation { cmd } => *cmd,
            })
            .collect();
        for name in crate::registry::toml_command_names() {
            all_cmds.insert(name);
        }
        let handled: HashSet<&str> = HANDLED_CMDS.iter().copied().collect();

        let missing: Vec<_> = handled.difference(&all_cmds).collect();
        assert!(missing.is_empty(), "not in registry: {missing:?}");

        let extra: Vec<_> = all_cmds.difference(&handled).collect();
        assert!(extra.is_empty(), "not in HANDLED_CMDS: {extra:?}");
    }

}
