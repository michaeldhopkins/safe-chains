# Ansible

### `ansible-config`
<p class="cmd-url"><a href="https://docs.ansible.com/ansible/latest/cli/ansible-config.html">https://docs.ansible.com/ansible/latest/cli/ansible-config.html</a></p>

- **dump**: Flags: --help, --only-changed, -h
- **list**: Flags: --help, -h
- **view**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -h

### `ansible-doc`
<p class="cmd-url"><a href="https://docs.ansible.com/ansible/latest/cli/ansible-doc.html">https://docs.ansible.com/ansible/latest/cli/ansible-doc.html</a></p>

- Allowed standalone flags: --help, --json, --list, --metadata-dump, --version, -F, -h, -j, -l
- Allowed valued flags: --module-path, --type, -M, -t
- Bare invocation allowed

### `ansible-galaxy`
<p class="cmd-url"><a href="https://docs.ansible.com/ansible/latest/cli/ansible-galaxy.html">https://docs.ansible.com/ansible/latest/cli/ansible-galaxy.html</a></p>

- **info**: Flags: --help, -h
- **list**: Flags: --help, -h
- **search**: Flags: --help, -h. Valued: --author, --galaxy-tags, --platforms
- Allowed standalone flags: --help, --version, -h

### `ansible-inventory`
<p class="cmd-url"><a href="https://docs.ansible.com/ansible/latest/cli/ansible-inventory.html">https://docs.ansible.com/ansible/latest/cli/ansible-inventory.html</a></p>

- **--graph**: Flags: --help, --vars, -h. Valued: --inventory, --limit, -i, -l
- **--host**: Flags: --help, -h. Valued: --inventory, -i
- **--list**: Flags: --help, --yaml, --toml, --export, -h, -y. Valued: --inventory, --limit, -i, -l
- Allowed standalone flags: --help, --version, -h

### `ansible-playbook`
<p class="cmd-url"><a href="https://docs.ansible.com/ansible/latest/cli/ansible-playbook.html">https://docs.ansible.com/ansible/latest/cli/ansible-playbook.html</a></p>

- Requires --list-hosts, --list-tasks, --list-tags, --syntax-check. - Allowed standalone flags: --help, --list-hosts, --list-tags, --list-tasks, --syntax-check, --version, -h
- Allowed valued flags: --connection, --extra-vars, --inventory, --limit, --tags, --skip-tags, -C, -c, -e, -i, -l, -t

