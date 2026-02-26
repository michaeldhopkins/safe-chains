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

    #[test]
    fn docker_ps() {
        assert!(check("docker ps"));
    }

    #[test]
    fn docker_ps_all() {
        assert!(check("docker ps -a"));
    }

    #[test]
    fn docker_images() {
        assert!(check("docker images"));
    }

    #[test]
    fn docker_logs() {
        assert!(check("docker logs container_name"));
    }

    #[test]
    fn docker_inspect() {
        assert!(check("docker inspect container_name"));
    }

    #[test]
    fn docker_info() {
        assert!(check("docker info"));
    }

    #[test]
    fn docker_version() {
        assert!(check("docker version"));
    }

    #[test]
    fn docker_top() {
        assert!(check("docker top container_name"));
    }

    #[test]
    fn docker_stats() {
        assert!(check("docker stats --no-stream"));
    }

    #[test]
    fn docker_history() {
        assert!(check("docker history image_name"));
    }

    #[test]
    fn docker_port() {
        assert!(check("docker port container_name"));
    }

    #[test]
    fn docker_diff() {
        assert!(check("docker diff container_name"));
    }

    #[test]
    fn docker_network_ls() {
        assert!(check("docker network ls"));
    }

    #[test]
    fn docker_network_inspect() {
        assert!(check("docker network inspect bridge"));
    }

    #[test]
    fn docker_volume_ls() {
        assert!(check("docker volume ls"));
    }

    #[test]
    fn docker_volume_inspect() {
        assert!(check("docker volume inspect my_vol"));
    }

    #[test]
    fn docker_container_ls() {
        assert!(check("docker container ls"));
    }

    #[test]
    fn docker_container_inspect() {
        assert!(check("docker container inspect my_container"));
    }

    #[test]
    fn docker_image_ls() {
        assert!(check("docker image ls"));
    }

    #[test]
    fn docker_image_inspect() {
        assert!(check("docker image inspect my_image"));
    }

    #[test]
    fn docker_system_info() {
        assert!(check("docker system info"));
    }

    #[test]
    fn docker_system_df() {
        assert!(check("docker system df"));
    }

    #[test]
    fn docker_compose_config() {
        assert!(check("docker compose config"));
    }

    #[test]
    fn docker_compose_ps() {
        assert!(check("docker compose ps"));
    }

    #[test]
    fn docker_compose_ls() {
        assert!(check("docker compose ls"));
    }

    #[test]
    fn docker_compose_images() {
        assert!(check("docker compose images"));
    }

    #[test]
    fn docker_context_ls() {
        assert!(check("docker context ls"));
    }

    #[test]
    fn docker_buildx_ls() {
        assert!(check("docker buildx ls"));
    }

    #[test]
    fn docker_buildx_version() {
        assert!(check("docker buildx version"));
    }

    #[test]
    fn podman_ps() {
        assert!(check("podman ps -a"));
    }

    #[test]
    fn podman_images() {
        assert!(check("podman images"));
    }

    #[test]
    fn podman_logs() {
        assert!(check("podman logs container_name"));
    }

    #[test]
    fn docker_run_denied() {
        assert!(!check("docker run ubuntu"));
    }

    #[test]
    fn docker_exec_denied() {
        assert!(!check("docker exec -it container bash"));
    }

    #[test]
    fn docker_rm_denied() {
        assert!(!check("docker rm container_name"));
    }

    #[test]
    fn docker_rmi_denied() {
        assert!(!check("docker rmi image_name"));
    }

    #[test]
    fn docker_build_denied() {
        assert!(!check("docker build ."));
    }

    #[test]
    fn docker_push_denied() {
        assert!(!check("docker push image_name"));
    }

    #[test]
    fn docker_pull_denied() {
        assert!(!check("docker pull ubuntu"));
    }

    #[test]
    fn docker_stop_denied() {
        assert!(!check("docker stop container_name"));
    }

    #[test]
    fn docker_kill_denied() {
        assert!(!check("docker kill container_name"));
    }

    #[test]
    fn docker_compose_up_denied() {
        assert!(!check("docker compose up"));
    }

    #[test]
    fn docker_compose_down_denied() {
        assert!(!check("docker compose down"));
    }

    #[test]
    fn docker_network_create_denied() {
        assert!(!check("docker network create my_net"));
    }

    #[test]
    fn docker_volume_create_denied() {
        assert!(!check("docker volume create my_vol"));
    }

    #[test]
    fn bare_docker_denied() {
        assert!(!check("docker"));
    }

    #[test]
    fn docker_version_flag() {
        assert!(check("docker --version"));
    }

    #[test]
    fn docker_compose_version_flag() {
        assert!(check("docker compose --version"));
    }

    #[test]
    fn docker_buildx_version_flag() {
        assert!(check("docker buildx --version"));
    }

    #[test]
    fn docker_run_version_bypass_denied() {
        assert!(!check("docker run evil --version"));
    }
}
