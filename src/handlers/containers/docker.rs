use crate::command::{CommandDef, SubDef};
use crate::verdict::{SafetyLevel, Verdict};
use crate::parse::{Token, WordSet};
use crate::policy::{FlagPolicy, FlagStyle};

static DOCKER_PS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all", "--help", "--last", "--latest", "--no-trunc",
        "--quiet", "--size",
        "-a", "-h", "-l", "-n", "-q", "-s",
    ]),
    valued: WordSet::flags(&["--filter", "--format", "-f"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DOCKER_IMAGES_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all", "--digests", "--help", "--no-trunc", "--quiet",
        "-a", "-h", "-q",
    ]),
    valued: WordSet::flags(&["--filter", "--format", "-f"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DOCKER_LOGS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--details", "--follow", "--help", "--timestamps",
        "-f", "-h", "-t",
    ]),
    valued: WordSet::flags(&["--since", "--tail", "--until", "-n"]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DOCKER_INSPECT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--size", "-h", "-s"]),
    valued: WordSet::flags(&["--format", "--type", "-f"]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DOCKER_INFO_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&["--format", "-f"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DOCKER_VERSION_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&["--format", "-f"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DOCKER_STATS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--all", "--help", "--no-stream", "--no-trunc", "-a", "-h"]),
    valued: WordSet::flags(&["--format"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DOCKER_HISTORY_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--human", "--no-trunc", "--quiet", "-H", "-h", "-q"]),
    valued: WordSet::flags(&["--format"]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DOCKER_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DOCKER_LS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--no-trunc", "--quiet", "-h", "-q"]),
    valued: WordSet::flags(&["--filter", "--format", "-f"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DOCKER_COMPOSE_PS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all", "--help", "--no-trunc", "--orphans", "--quiet",
        "--services",
        "-a", "-h", "-q",
    ]),
    valued: WordSet::flags(&["--filter", "--format", "--status"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DOCKER_COMPOSE_CONFIG_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--dry-run", "--hash", "--help", "--images", "--no-consistency",
        "--no-interpolate", "--no-normalize", "--no-path-resolution",
        "--profiles", "--quiet", "--resolve-image-digests",
        "--services", "--volumes",
        "-h", "-q",
    ]),
    valued: WordSet::flags(&["--format", "--output", "-o"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

fn check_docker_version_flag(tokens: &[Token]) -> Verdict {
    if tokens.len() == 1 { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied }

}

static DOCKER_SUBS: &[SubDef] = &[
    SubDef::Nested { name: "buildx", subs: &[
        SubDef::Custom { name: "--version", check: check_docker_version_flag, doc: "", test_suffix: None },
        SubDef::Policy { name: "inspect", policy: &DOCKER_SIMPLE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "ls", policy: &DOCKER_SIMPLE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "version", policy: &DOCKER_SIMPLE_POLICY, level: SafetyLevel::Inert },
    ]},
    SubDef::Nested { name: "compose", subs: &[
        SubDef::Custom { name: "--version", check: check_docker_version_flag, doc: "", test_suffix: None },
        SubDef::Policy { name: "config", policy: &DOCKER_COMPOSE_CONFIG_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "images", policy: &DOCKER_SIMPLE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "ls", policy: &DOCKER_SIMPLE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "ps", policy: &DOCKER_COMPOSE_PS_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "top", policy: &DOCKER_SIMPLE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "version", policy: &DOCKER_SIMPLE_POLICY, level: SafetyLevel::Inert },
    ]},
    SubDef::Nested { name: "container", subs: &[
        SubDef::Policy { name: "diff", policy: &DOCKER_SIMPLE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "inspect", policy: &DOCKER_INSPECT_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "list", policy: &DOCKER_PS_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "logs", policy: &DOCKER_LOGS_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "ls", policy: &DOCKER_PS_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "port", policy: &DOCKER_SIMPLE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "stats", policy: &DOCKER_STATS_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "top", policy: &DOCKER_SIMPLE_POLICY, level: SafetyLevel::Inert },
    ]},
    SubDef::Nested { name: "context", subs: &[
        SubDef::Policy { name: "inspect", policy: &DOCKER_LS_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "ls", policy: &DOCKER_LS_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "show", policy: &DOCKER_LS_POLICY, level: SafetyLevel::Inert },
    ]},
    SubDef::Policy { name: "diff", policy: &DOCKER_SIMPLE_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "history", policy: &DOCKER_HISTORY_POLICY, level: SafetyLevel::Inert },
    SubDef::Nested { name: "image", subs: &[
        SubDef::Policy { name: "history", policy: &DOCKER_HISTORY_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "inspect", policy: &DOCKER_INSPECT_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "list", policy: &DOCKER_IMAGES_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "ls", policy: &DOCKER_IMAGES_POLICY, level: SafetyLevel::Inert },
    ]},
    SubDef::Policy { name: "images", policy: &DOCKER_IMAGES_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "info", policy: &DOCKER_INFO_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "inspect", policy: &DOCKER_INSPECT_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "logs", policy: &DOCKER_LOGS_POLICY, level: SafetyLevel::Inert },
    SubDef::Nested { name: "manifest", subs: &[
        SubDef::Policy { name: "inspect", policy: &DOCKER_INSPECT_POLICY, level: SafetyLevel::Inert },
    ]},
    SubDef::Nested { name: "network", subs: &[
        SubDef::Policy { name: "inspect", policy: &DOCKER_LS_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "ls", policy: &DOCKER_LS_POLICY, level: SafetyLevel::Inert },
    ]},
    SubDef::Policy { name: "port", policy: &DOCKER_SIMPLE_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "ps", policy: &DOCKER_PS_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "stats", policy: &DOCKER_STATS_POLICY, level: SafetyLevel::Inert },
    SubDef::Nested { name: "system", subs: &[
        SubDef::Policy { name: "df", policy: &DOCKER_INFO_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "info", policy: &DOCKER_INFO_POLICY, level: SafetyLevel::Inert },
    ]},
    SubDef::Policy { name: "top", policy: &DOCKER_SIMPLE_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "version", policy: &DOCKER_VERSION_POLICY, level: SafetyLevel::Inert },
    SubDef::Nested { name: "volume", subs: &[
        SubDef::Policy { name: "inspect", policy: &DOCKER_LS_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "ls", policy: &DOCKER_LS_POLICY, level: SafetyLevel::Inert },
    ]},
];

pub(crate) static DOCKER: CommandDef = CommandDef {
    name: "docker",
    subs: DOCKER_SUBS,
    bare_flags: &["--help", "--version", "-V", "-h"],
    url: "https://docs.docker.com/reference/cli/docker/",
    aliases: &[],
};

pub(crate) static PODMAN: CommandDef = CommandDef {
    name: "podman",
    subs: DOCKER_SUBS,
    bare_flags: &["--help", "--version", "-V", "-h"],
    url: "https://docs.podman.io/en/latest/Commands.html",
    aliases: &[],
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        docker_ps: "docker ps",
        docker_ps_all: "docker ps -a",
        docker_ps_quiet: "docker ps -q",
        docker_ps_filter: "docker ps --filter status=running",
        docker_ps_format: "docker ps --format '{{.Names}}'",
        docker_images: "docker images",
        docker_images_quiet: "docker images -q",
        docker_images_all: "docker images --all",
        docker_logs: "docker logs container_name",
        docker_logs_follow: "docker logs -f container_name",
        docker_logs_tail: "docker logs --tail 100 container_name",
        docker_inspect: "docker inspect container_name",
        docker_inspect_format: "docker inspect --format '{{.State.Status}}' c",
        docker_info: "docker info",
        docker_info_format: "docker info --format '{{.ServerVersion}}'",
        docker_version: "docker version",
        docker_version_format: "docker version --format '{{.Server.Version}}'",
        docker_top: "docker top container_name",
        docker_stats: "docker stats --no-stream",
        docker_stats_all: "docker stats --all --no-stream",
        docker_stats_format: "docker stats --format '{{.Name}}' --no-stream",
        docker_history: "docker history image_name",
        docker_history_no_trunc: "docker history --no-trunc image_name",
        docker_port: "docker port container_name",
        docker_diff: "docker diff container_name",
        docker_network_ls: "docker network ls",
        docker_network_ls_filter: "docker network ls --filter driver=bridge",
        docker_network_inspect: "docker network inspect bridge",
        docker_volume_ls: "docker volume ls",
        docker_volume_ls_quiet: "docker volume ls -q",
        docker_volume_inspect: "docker volume inspect my_vol",
        docker_container_ls: "docker container ls",
        docker_container_ls_all: "docker container ls -a",
        docker_container_inspect: "docker container inspect my_container",
        docker_container_logs: "docker container logs -f my_container",
        docker_image_ls: "docker image ls",
        docker_image_ls_quiet: "docker image ls -q",
        docker_image_inspect: "docker image inspect my_image",
        docker_image_history: "docker image history my_image",
        docker_system_info: "docker system info",
        docker_system_df: "docker system df",
        docker_compose_config: "docker compose config",
        docker_compose_config_services: "docker compose config --services",
        docker_compose_ps: "docker compose ps",
        docker_compose_ps_quiet: "docker compose ps -q",
        docker_compose_ls: "docker compose ls",
        docker_compose_images: "docker compose images",
        docker_compose_top: "docker compose top",
        docker_context_ls: "docker context ls",
        docker_context_inspect: "docker context inspect default",
        docker_buildx_ls: "docker buildx ls",
        docker_buildx_version: "docker buildx version",
        docker_buildx_inspect: "docker buildx inspect",
        podman_ps: "podman ps -a",
        podman_images: "podman images",
        podman_logs: "podman logs container_name",
        docker_version_flag: "docker --version",
        docker_compose_version_flag: "docker compose --version",
        docker_buildx_version_flag: "docker buildx --version",
    }

    denied! {
        docker_run_version_bypass_denied: "docker run evil --version",
    }
}
