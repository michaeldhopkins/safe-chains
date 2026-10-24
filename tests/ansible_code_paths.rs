//! Places ansible loads code from. An inventory source that is an executable script is run to list
//! the hosts, and the plugin, module, role and collection folders hold Python ansible imports. Every
//! ansible command that reads its inventory or loads a plugin is checked against each, by flag and
//! by environment variable: pointed outside the project, it asks.

use safe_chains::pathctx::{PathCtx, enter};

fn approved(command: &str) -> bool {
    let root = env!("CARGO_MANIFEST_DIR").to_string();
    let _ctx = enter(PathCtx { cwd: Some(root.clone()), root: Some(root), session_id: None });
    safe_chains::is_safe_command(command)
}

const INVENTORY_READERS: [&str; 7] = [
    "ansible-inventory --list", "ansible-inventory --graph", "ansible-inventory --host web1", "ansible-playbook --syntax-check site.yml",
    "ansible-playbook --list-hosts site.yml", "ansible-playbook --list-tasks site.yml", "ansible-playbook --list-tags site.yml",
];

const FOREIGN: [&str; 3] = ["/tmp/inv.py", "/private/tmp/x/inventory.sh", "../../elsewhere/inv.py"];

#[test]
fn an_inventory_source_outside_the_project_asks() {
    for command in INVENTORY_READERS {
        assert!(approved(command), "precondition: `{command}`");
        for value in FOREIGN {
            for flag in [format!("-i {value}"), format!("--inventory {value}"), format!("--inventory={value}")] {
                let line = format!("{command} {flag}");
                assert!(!approved(&line), "`{line}` was approved");
            }
        }
        for value in ["./inventory/hosts.yml", "inventory.ini", "web1,web2,"] {
            let line = format!("{command} -i {value}");
            assert!(approved(&line), "`{line}` should stay approved");
        }
    }
}

const CODE_FOLDERS: [&str; 23] = [
    "ANSIBLE_ACTION_PLUGINS", "ANSIBLE_BECOME_PLUGINS", "ANSIBLE_CACHE_PLUGINS", "ANSIBLE_CALLBACK_PLUGINS", "ANSIBLE_CLICONF_PLUGINS",
    "ANSIBLE_COLLECTIONS_PATH", "ANSIBLE_COLLECTIONS_PATHS", "ANSIBLE_CONNECTION_PLUGINS", "ANSIBLE_DOC_FRAGMENT_PLUGINS",
    "ANSIBLE_FILTER_PLUGINS", "ANSIBLE_HOME", "ANSIBLE_HTTPAPI_PLUGINS", "ANSIBLE_INVENTORY_PLUGINS", "ANSIBLE_LIBRARY",
    "ANSIBLE_LOOKUP_PLUGINS", "ANSIBLE_MODULE_UTILS", "ANSIBLE_NETCONF_PLUGINS", "ANSIBLE_PLAYBOOK_DIR", "ANSIBLE_ROLES_PATH",
    "ANSIBLE_STRATEGY_PLUGINS", "ANSIBLE_TERMINAL_PLUGINS", "ANSIBLE_TEST_PLUGINS", "ANSIBLE_VARS_PLUGINS",
];

#[test]
fn a_plugin_folder_outside_the_project_asks() {
    let inside = format!("{}/plugins", env!("CARGO_MANIFEST_DIR"));
    for command in INVENTORY_READERS {
        for name in CODE_FOLDERS {
            for value in ["/tmp/evil", "/private/tmp/x", "../../elsewhere", "./plugins:/tmp/evil"] {
                let line = format!("{name}={value} {command}");
                assert!(!approved(&line), "`{line}` was approved");
            }
            let line = format!("{name}={inside} {command}");
            assert!(approved(&line), "`{line}` should stay approved");
        }
    }
}

#[test]
fn an_inventory_named_by_the_environment_asks() {
    for command in INVENTORY_READERS {
        for value in ["/tmp/inv.py", "./hosts.ini,/tmp/inv.py", "./hosts.ini"] {
            let line = format!("ANSIBLE_INVENTORY={value} {command}");
            assert!(!approved(&line), "`{line}` was approved");
        }
    }
}
