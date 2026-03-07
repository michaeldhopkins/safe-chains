use crate::command::{CommandDef, SubDef};
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static KUBECTL_GET_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all-namespaces", "--no-headers", "--show-labels", "--watch",
    ]),
    standalone_short: b"Aw",
    valued: WordSet::flags(&[
        "--field-selector", "--label-selector", "--namespace",
        "--output", "--selector", "--sort-by",
    ]),
    valued_short: b"lno",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static KUBECTL_DESCRIBE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all-namespaces", "--show-events",
    ]),
    standalone_short: b"A",
    valued: WordSet::flags(&["--namespace", "--selector"]),
    valued_short: b"ln",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static KUBECTL_LOGS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all-containers", "--follow", "--previous", "--timestamps",
    ]),
    standalone_short: b"fp",
    valued: WordSet::flags(&["--container", "--namespace", "--since", "--tail"]),
    valued_short: b"cn",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static KUBECTL_TOP_NODE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--no-headers"]),
    standalone_short: b"",
    valued: WordSet::flags(&["--selector", "--sort-by"]),
    valued_short: b"l",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static KUBECTL_TOP_POD_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all-namespaces", "--containers", "--no-headers",
    ]),
    standalone_short: b"A",
    valued: WordSet::flags(&["--namespace", "--selector", "--sort-by"]),
    valued_short: b"ln",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static KUBECTL_EXPLAIN_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--recursive"]),
    standalone_short: b"",
    valued: WordSet::flags(&["--api-version"]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static KUBECTL_API_RESOURCES_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--namespaced", "--no-headers"]),
    standalone_short: b"",
    valued: WordSet::flags(&["--api-group", "--output", "--sort-by", "--verbs"]),
    valued_short: b"o",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static KUBECTL_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    standalone_short: b"",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static KUBECTL_VERSION_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--client", "--short"]),
    standalone_short: b"",
    valued: WordSet::flags(&["--output"]),
    valued_short: b"o",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static KUBECTL_CONFIG_VIEW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--flatten", "--minify", "--raw"]),
    standalone_short: b"",
    valued: WordSet::flags(&["--output"]),
    valued_short: b"o",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static KUBECTL_CONFIG_GET_CONTEXTS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--no-headers"]),
    standalone_short: b"",
    valued: WordSet::flags(&["--output"]),
    valued_short: b"o",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static KUBECTL_AUTH_CAN_I_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    standalone_short: b"",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static KUBECTL_EVENTS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--all-namespaces", "--watch"]),
    standalone_short: b"Aw",
    valued: WordSet::flags(&["--for", "--namespace", "--output", "--types"]),
    valued_short: b"no",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static KUBECTL: CommandDef = CommandDef {
    name: "kubectl",
    subs: &[
        SubDef::Policy { name: "api-resources", policy: &KUBECTL_API_RESOURCES_POLICY },
        SubDef::Policy { name: "api-versions", policy: &KUBECTL_BARE_POLICY },
        SubDef::Nested { name: "auth", subs: &[
            SubDef::Policy { name: "can-i", policy: &KUBECTL_AUTH_CAN_I_POLICY },
            SubDef::Policy { name: "whoami", policy: &KUBECTL_BARE_POLICY },
        ]},
        SubDef::Policy { name: "cluster-info", policy: &KUBECTL_BARE_POLICY },
        SubDef::Nested { name: "config", subs: &[
            SubDef::Policy { name: "current-context", policy: &KUBECTL_BARE_POLICY },
            SubDef::Policy { name: "get-contexts", policy: &KUBECTL_CONFIG_GET_CONTEXTS_POLICY },
            SubDef::Policy { name: "view", policy: &KUBECTL_CONFIG_VIEW_POLICY },
        ]},
        SubDef::Policy { name: "describe", policy: &KUBECTL_DESCRIBE_POLICY },
        SubDef::Policy { name: "events", policy: &KUBECTL_EVENTS_POLICY },
        SubDef::Policy { name: "explain", policy: &KUBECTL_EXPLAIN_POLICY },
        SubDef::Policy { name: "get", policy: &KUBECTL_GET_POLICY },
        SubDef::Policy { name: "logs", policy: &KUBECTL_LOGS_POLICY },
        SubDef::Nested { name: "top", subs: &[
            SubDef::Policy { name: "node", policy: &KUBECTL_TOP_NODE_POLICY },
            SubDef::Policy { name: "pod", policy: &KUBECTL_TOP_POD_POLICY },
        ]},
        SubDef::Policy { name: "version", policy: &KUBECTL_VERSION_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
    url: "https://kubernetes.io/docs/reference/kubectl/",
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        kubectl_get_pods: "kubectl get pods",
        kubectl_get_all_ns: "kubectl get pods --all-namespaces",
        kubectl_get_output: "kubectl get pods -o json",
        kubectl_get_labels: "kubectl get pods --show-labels",
        kubectl_get_watch: "kubectl get pods -w",
        kubectl_get_selector: "kubectl get pods -l app=web",
        kubectl_get_no_headers: "kubectl get pods --no-headers",
        kubectl_describe_pod: "kubectl describe pod my-pod",
        kubectl_describe_ns: "kubectl describe pod my-pod -n kube-system",
        kubectl_logs_pod: "kubectl logs my-pod",
        kubectl_logs_follow: "kubectl logs -f my-pod",
        kubectl_logs_tail: "kubectl logs --tail 100 my-pod",
        kubectl_logs_previous: "kubectl logs -p my-pod",
        kubectl_logs_container: "kubectl logs my-pod -c nginx",
        kubectl_top_node: "kubectl top node",
        kubectl_top_pod: "kubectl top pod",
        kubectl_top_pod_ns: "kubectl top pod -n default",
        kubectl_explain: "kubectl explain pods",
        kubectl_explain_recursive: "kubectl explain pods --recursive",
        kubectl_api_resources: "kubectl api-resources",
        kubectl_api_resources_output: "kubectl api-resources -o wide",
        kubectl_api_versions: "kubectl api-versions",
        kubectl_cluster_info: "kubectl cluster-info",
        kubectl_version: "kubectl version",
        kubectl_version_client: "kubectl version --client",
        kubectl_version_short: "kubectl version --short",
        kubectl_config_view: "kubectl config view",
        kubectl_config_view_minify: "kubectl config view --minify",
        kubectl_config_get_contexts: "kubectl config get-contexts",
        kubectl_config_current_context: "kubectl config current-context",
        kubectl_auth_can_i: "kubectl auth can-i get pods",
        kubectl_auth_whoami: "kubectl auth whoami",
        kubectl_events: "kubectl events",
        kubectl_events_ns: "kubectl events -n default",
        kubectl_events_watch: "kubectl events -w",
        kubectl_help: "kubectl --help",
    }

    denied! {
        kubectl_delete: "kubectl delete pod my-pod",
        kubectl_apply: "kubectl apply -f deploy.yaml",
        kubectl_exec: "kubectl exec -it my-pod -- bash",
        kubectl_bare: "kubectl",
    }
}
