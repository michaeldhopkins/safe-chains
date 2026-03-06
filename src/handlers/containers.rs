use crate::parse::{Segment, Token, WordSet};
use crate::policy::{self, FlagPolicy, FlagStyle};

static DOCKER_PS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all", "--last", "--latest", "--no-trunc",
        "--quiet", "--size",
    ]),
    standalone_short: b"alnoqs",
    valued: WordSet::new(&["--filter", "--format"]),
    valued_short: b"fn",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DOCKER_IMAGES_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all", "--digests", "--no-trunc", "--quiet",
    ]),
    standalone_short: b"aq",
    valued: WordSet::new(&["--filter", "--format"]),
    valued_short: b"f",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DOCKER_LOGS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--details", "--follow", "--timestamps",
    ]),
    standalone_short: b"ft",
    valued: WordSet::new(&["--since", "--tail", "--until"]),
    valued_short: b"n",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DOCKER_INSPECT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--size"]),
    standalone_short: b"s",
    valued: WordSet::new(&["--format", "--type"]),
    valued_short: b"f",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DOCKER_INFO_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&["--format"]),
    valued_short: b"f",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DOCKER_VERSION_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&["--format"]),
    valued_short: b"f",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DOCKER_STATS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--all", "--no-stream", "--no-trunc"]),
    standalone_short: b"a",
    valued: WordSet::new(&["--format"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DOCKER_HISTORY_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--human", "--no-trunc", "--quiet"]),
    standalone_short: b"Hq",
    valued: WordSet::new(&["--format"]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DOCKER_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DOCKER_LS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--no-trunc", "--quiet"]),
    standalone_short: b"q",
    valued: WordSet::new(&["--filter", "--format"]),
    valued_short: b"f",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DOCKER_COMPOSE_PS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all", "--no-trunc", "--orphans", "--quiet",
        "--services",
    ]),
    standalone_short: b"aq",
    valued: WordSet::new(&["--filter", "--format", "--status"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DOCKER_COMPOSE_CONFIG_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--dry-run", "--hash", "--images", "--no-consistency",
        "--no-interpolate", "--no-normalize", "--no-path-resolution",
        "--profiles", "--quiet", "--resolve-image-digests",
        "--services", "--volumes",
    ]),
    standalone_short: b"q",
    valued: WordSet::new(&["--format", "--output"]),
    valued_short: b"o",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

fn check_docker_subcmd(tokens: &[Token], subcmd_pos: usize) -> bool {
    if subcmd_pos >= tokens.len() {
        return false;
    }
    let subcmd = tokens[subcmd_pos].as_str();
    let rest = &tokens[subcmd_pos..];
    let policy = match subcmd {
        "ps" => &DOCKER_PS_POLICY,
        "images" => &DOCKER_IMAGES_POLICY,
        "logs" => &DOCKER_LOGS_POLICY,
        "inspect" => &DOCKER_INSPECT_POLICY,
        "info" => &DOCKER_INFO_POLICY,
        "version" => &DOCKER_VERSION_POLICY,
        "top" | "port" | "diff" => &DOCKER_SIMPLE_POLICY,
        "stats" => &DOCKER_STATS_POLICY,
        "history" => &DOCKER_HISTORY_POLICY,
        _ => return false,
    };
    policy::check(rest, policy)
}

fn check_docker_multi(tokens: &[Token]) -> bool {
    if tokens.len() < 3 {
        return false;
    }
    let group = tokens[1].as_str();
    let action = tokens[2].as_str();
    let rest = &tokens[2..];

    match group {
        "compose" => match action {
            "--version" => tokens.len() == 3,
            "ps" => policy::check(rest, &DOCKER_COMPOSE_PS_POLICY),
            "config" => policy::check(rest, &DOCKER_COMPOSE_CONFIG_POLICY),
            "images" | "ls" | "top" | "version" => {
                policy::check(rest, &DOCKER_SIMPLE_POLICY)
            }
            _ => false,
        },
        "container" => match action {
            "diff" | "inspect" | "list" | "logs" | "ls"
            | "port" | "stats" | "top" => {
                let mapped = match action {
                    "list" | "ls" => &DOCKER_PS_POLICY,
                    "logs" => &DOCKER_LOGS_POLICY,
                    "inspect" => &DOCKER_INSPECT_POLICY,
                    "stats" => &DOCKER_STATS_POLICY,
                    _ => &DOCKER_SIMPLE_POLICY,
                };
                policy::check(rest, mapped)
            }
            _ => false,
        },
        "image" => match action {
            "history" | "inspect" | "list" | "ls" => {
                let mapped = match action {
                    "list" | "ls" => &DOCKER_IMAGES_POLICY,
                    "inspect" => &DOCKER_INSPECT_POLICY,
                    "history" => &DOCKER_HISTORY_POLICY,
                    _ => &DOCKER_SIMPLE_POLICY,
                };
                policy::check(rest, mapped)
            }
            _ => false,
        },
        "context" => match action {
            "inspect" | "ls" | "show" => policy::check(rest, &DOCKER_LS_POLICY),
            _ => false,
        },
        "network" | "volume" => match action {
            "inspect" | "ls" => policy::check(rest, &DOCKER_LS_POLICY),
            _ => false,
        },
        "system" => match action {
            "df" | "info" => policy::check(rest, &DOCKER_INFO_POLICY),
            _ => false,
        },
        "buildx" => match action {
            "--version" => tokens.len() == 3,
            "inspect" | "ls" | "version" => policy::check(rest, &DOCKER_SIMPLE_POLICY),
            _ => false,
        },
        "manifest" => match action {
            "inspect" => policy::check(rest, &DOCKER_INSPECT_POLICY),
            _ => false,
        },
        _ => false,
    }
}

pub fn is_safe_docker(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if check_docker_subcmd(tokens, 1) {
        return true;
    }
    check_docker_multi(tokens)
}

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    match cmd {
        "docker" | "podman" => Some(is_safe_docker(tokens)),
        _ => None,
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![CommandDoc::handler("docker / podman",
        "Top-level: diff, history, images, info, inspect, logs, port, ps, stats, top, version. \
         Multi-level: buildx, compose, container, context, image, manifest, network, system, volume. \
        ")]
}

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
        docker_run_denied: "docker run ubuntu",
        docker_exec_denied: "docker exec -it container bash",
        docker_rm_denied: "docker rm container_name",
        docker_rmi_denied: "docker rmi image_name",
        docker_build_denied: "docker build .",
        docker_push_denied: "docker push image_name",
        docker_pull_denied: "docker pull ubuntu",
        docker_stop_denied: "docker stop container_name",
        docker_kill_denied: "docker kill container_name",
        docker_compose_up_denied: "docker compose up",
        docker_compose_down_denied: "docker compose down",
        docker_network_create_denied: "docker network create my_net",
        docker_volume_create_denied: "docker volume create my_vol",
        bare_docker_denied: "docker",
        docker_run_version_bypass_denied: "docker run evil --version",
        docker_ps_unknown_denied: "docker ps --unknown-flag",
        docker_logs_unknown_denied: "docker logs --unknown container",
        docker_images_unknown_denied: "docker images --unknown",
        docker_network_ls_unknown_denied: "docker network ls --unknown",
        docker_compose_ps_unknown_denied: "docker compose ps --unknown",
    }
}
