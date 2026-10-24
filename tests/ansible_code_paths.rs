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
