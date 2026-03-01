use crate::parse::{Token, WordSet};

static DOCKER_READ_ONLY: WordSet = WordSet::new(&[
    "--version", "diff", "history", "images", "info", "inspect", "logs",
    "port", "ps", "stats", "top", "version",
]);

static DOCKER_MULTI: &[(&str, WordSet)] = &[
    ("buildx", WordSet::new(&["--version", "inspect", "ls", "version"])),
    ("compose", WordSet::new(&["--version", "config", "images", "ls", "ps", "top", "version"])),
    ("container", WordSet::new(&["diff", "inspect", "list", "logs", "ls", "port", "stats", "top"])),
    ("context", WordSet::new(&["inspect", "ls", "show"])),
    ("image", WordSet::new(&["history", "inspect", "list", "ls"])),
    ("manifest", WordSet::new(&["inspect"])),
    ("network", WordSet::new(&["inspect", "ls"])),
    ("system", WordSet::new(&["df", "info"])),
    ("volume", WordSet::new(&["inspect", "ls"])),
];

pub fn is_safe_docker(tokens: &[Token]) -> bool {
    super::is_safe_subcmd(tokens, &DOCKER_READ_ONLY, DOCKER_MULTI)
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![CommandDoc::wordset_multi("docker / podman", &DOCKER_READ_ONLY, DOCKER_MULTI)]
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
        docker_images: "docker images",
        docker_logs: "docker logs container_name",
        docker_inspect: "docker inspect container_name",
        docker_info: "docker info",
        docker_version: "docker version",
        docker_top: "docker top container_name",
        docker_stats: "docker stats --no-stream",
        docker_history: "docker history image_name",
        docker_port: "docker port container_name",
        docker_diff: "docker diff container_name",
        docker_network_ls: "docker network ls",
        docker_network_inspect: "docker network inspect bridge",
        docker_volume_ls: "docker volume ls",
        docker_volume_inspect: "docker volume inspect my_vol",
        docker_container_ls: "docker container ls",
        docker_container_inspect: "docker container inspect my_container",
        docker_image_ls: "docker image ls",
        docker_image_inspect: "docker image inspect my_image",
        docker_system_info: "docker system info",
        docker_system_df: "docker system df",
        docker_compose_config: "docker compose config",
        docker_compose_ps: "docker compose ps",
        docker_compose_ls: "docker compose ls",
        docker_compose_images: "docker compose images",
        docker_context_ls: "docker context ls",
        docker_buildx_ls: "docker buildx ls",
        docker_buildx_version: "docker buildx version",
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
    }
}
