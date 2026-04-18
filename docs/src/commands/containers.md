# Containers

### `cosign`
<p class="cmd-url"><a href="https://github.com/sigstore/cosign">https://github.com/sigstore/cosign</a></p>

- **tree**: Flags: --help, -h
- **triangulate**: Flags: --help, -h. Valued: --type
- **verify**: Flags: --check-claims, --help, --local-image, --offline, -h. Valued: --attachment, --certificate, --certificate-chain, --certificate-identity, --certificate-identity-regexp, --certificate-oidc-issuer, --certificate-oidc-issuer-regexp, --k8s-keychain, --key, --output, --payload, --rekor-url, --signature, --slot, -o
- **verify-attestation**: Flags: --check-claims, --help, --local-image, --offline, -h. Valued: --certificate, --certificate-identity, --certificate-oidc-issuer, --key, --output, --policy, --rekor-url, --type, -o
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -h

### `crane`
<p class="cmd-url"><a href="https://github.com/google/go-containerregistry">https://github.com/google/go-containerregistry</a></p>

- **blob**: Flags: --help, -h
- **catalog**: Flags: --full-ref, --help, -h
- **config**: Flags: --help, -h
- **digest**: Flags: --full-ref, --help, --tarball, -h
- **ls**: Flags: --full-ref, --help, --omit-digest-tags, -h
- **manifest**: Flags: --help, -h
- **validate**: Flags: --fast, --help, --remote, --tarball, -h
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, -h

### `docker`
<p class="cmd-url"><a href="https://docs.docker.com/reference/cli/docker/">https://docs.docker.com/reference/cli/docker/</a></p>

- **buildx --version**
- **buildx inspect**: Flags: --help, -h
- **buildx ls**: Flags: --help, -h
- **buildx version**: Flags: --help, -h
- **compose --version**
- **compose config**: Flags: --dry-run, --hash, --help, --images, --no-consistency, --no-interpolate, --no-normalize, --no-path-resolution, --profiles, --quiet, --resolve-image-digests, --services, --volumes, -h, -q. Valued: --format, --output, -o
- **compose images**: Flags: --help, -h
- **compose ls**: Flags: --help, -h
- **compose port**: Flags: --help, --index, --protocol, -h
- **compose ps**: Flags: --all, --help, --no-trunc, --orphans, --quiet, --services, -a, -h, -q. Valued: --filter, --format, --status
- **compose top**: Flags: --help, -h
- **compose version**: Flags: --help, -h
- **container diff**: Flags: --help, -h
- **container inspect**: Flags: --help, --size, -h, -s. Valued: --format, --type, -f
- **container list**: Flags: --all, --help, --last, --latest, --no-trunc, --quiet, --size, -a, -h, -l, -n, -q, -s. Valued: --filter, --format, -f
- **container logs**: Flags: --details, --follow, --help, --timestamps, -f, -h, -t. Valued: --since, --tail, --until, -n
- **container ls**: Flags: --all, --help, --last, --latest, --no-trunc, --quiet, --size, -a, -h, -l, -n, -q, -s. Valued: --filter, --format, -f
- **container port**: Flags: --help, -h
- **container stats**: Flags: --all, --help, --no-stream, --no-trunc, -a, -h. Valued: --format
- **container top**: Flags: --help, -h
- **context inspect**: Flags: --help, --no-trunc, --quiet, -h, -q. Valued: --filter, --format, -f
- **context ls**: Flags: --help, --no-trunc, --quiet, -h, -q. Valued: --filter, --format, -f
- **context show**: Flags: --help, --no-trunc, --quiet, -h, -q. Valued: --filter, --format, -f
- **diff**: Flags: --help, -h
- **history**: Flags: --help, --human, --no-trunc, --quiet, -H, -h, -q. Valued: --format
- **image history**: Flags: --help, --human, --no-trunc, --quiet, -H, -h, -q. Valued: --format
- **image inspect**: Flags: --help, --size, -h, -s. Valued: --format, --type, -f
- **image list**: Flags: --all, --digests, --help, --no-trunc, --quiet, -a, -h, -q. Valued: --filter, --format, -f
- **image ls**: Flags: --all, --digests, --help, --no-trunc, --quiet, -a, -h, -q. Valued: --filter, --format, -f
- **images**: Flags: --all, --digests, --help, --no-trunc, --quiet, -a, -h, -q. Valued: --filter, --format, -f
- **info**: Flags: --help, -h. Valued: --format, -f
- **inspect**: Flags: --help, --size, -h, -s. Valued: --format, --type, -f
- **logs**: Flags: --details, --follow, --help, --timestamps, -f, -h, -t. Valued: --since, --tail, --until, -n
- **manifest inspect**: Flags: --help, --size, -h, -s. Valued: --format, --type, -f
- **network inspect**: Flags: --help, --no-trunc, --quiet, -h, -q. Valued: --filter, --format, -f
- **network ls**: Flags: --help, --no-trunc, --quiet, -h, -q. Valued: --filter, --format, -f
- **port**: Flags: --help, -h
- **ps**: Flags: --all, --help, --last, --latest, --no-trunc, --quiet, --size, -a, -h, -l, -n, -q, -s. Valued: --filter, --format, -f
- **stats**: Flags: --all, --help, --no-stream, --no-trunc, -a, -h. Valued: --format
- **system df**: Flags: --help, -h. Valued: --format, -f
- **system info**: Flags: --help, -h. Valued: --format, -f
- **top**: Flags: --help, -h
- **version**: Flags: --help, -h. Valued: --format, -f
- **volume inspect**: Flags: --help, --no-trunc, --quiet, -h, -q. Valued: --filter, --format, -f
- **volume ls**: Flags: --help, --no-trunc, --quiet, -h, -q. Valued: --filter, --format, -f
- Allowed standalone flags: --help, --version, -V, -h

### `helm`
<p class="cmd-url"><a href="https://helm.sh/docs/helm/">https://helm.sh/docs/helm/</a></p>

- **env**: Flags: --help, -h
- **get all**: Flags: --help, -h. Valued: --output, --revision, --template, -o
- **get hooks**: Flags: --help, -h. Valued: --output, --revision, -o
- **get manifest**: Flags: --help, -h. Valued: --output, --revision, -o
- **get metadata**: Flags: --help, -h. Valued: --output, --revision, -o
- **get notes**: Flags: --help, -h. Valued: --output, --revision, -o
- **get values**: Flags: --help, -h. Valued: --output, --revision, -o
- **history**: Flags: --help, -h. Valued: --max, --output, -o
- **lint**: Flags: --help, --quiet, --strict, --with-subcharts, -h. Valued: --set, --set-file, --set-json, --set-string, --values, -f
- **list**: Flags: --all, --all-namespaces, --deployed, --failed, --help, --pending, --reverse, --short, --superseded, --uninstalled, --uninstalling, -A, -a, -h, -q. Valued: --filter, --max, --offset, --output, --time-format, -o
- **search hub**: Flags: --help, --list-repo-url, -h. Valued: --max-col-width, --output, -o
- **search repo**: Flags: --devel, --help, --regexp, --versions, -h, -l, -r. Valued: --max-col-width, --output, --version, -o
- **show all**: Flags: --devel, --help, -h. Valued: --ca-file, --cert-file, --key-file, --keyring, --repo, --username, --version
- **show chart**: Flags: --devel, --help, -h. Valued: --repo, --version
- **show crds**: Flags: --devel, --help, -h. Valued: --repo, --version
- **show readme**: Flags: --devel, --help, -h. Valued: --repo, --version
- **show values**: Flags: --devel, --help, -h. Valued: --jsonpath, --repo, --version
- **status**: Flags: --help, --show-desc, --show-resources, -h. Valued: --output, --revision, -o
- **template**: Flags: --api-versions, --create-namespace, --dependency-update, --devel, --dry-run, --generate-name, --help, --include-crds, --is-upgrade, --no-hooks, --release-name, --replace, --skip-crds, --skip-tests, --validate, -g, -h. Valued: --kube-version, --name-template, --namespace, --output-dir, --post-renderer, --repo, --set, --set-file, --set-json, --set-string, --show-only, --timeout, --values, --version, -f, -n, -s
- **verify**: Flags: --help, -h. Valued: --keyring
- **version**: Flags: --help, --short, --template, -h
- Allowed standalone flags: --help, -h

### `kind`
<p class="cmd-url"><a href="https://kind.sigs.k8s.io/">https://kind.sigs.k8s.io/</a></p>

- **get clusters**: Flags: --help, -h
- **get kubeconfig**: Flags: --help, --internal, -h. Valued: --name
- **get nodes**: Flags: --help, -h. Valued: --name
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -h

### `kubectl`
<p class="cmd-url"><a href="https://kubernetes.io/docs/reference/kubectl/">https://kubernetes.io/docs/reference/kubectl/</a></p>

- **api-resources**: Flags: --help, --namespaced, --no-headers, -h. Valued: --api-group, --output, --sort-by, --verbs, -o
- **api-versions**: Flags: --help, -h
- **auth can-i**: Flags: --help, -h
- **auth whoami**: Flags: --help, -h
- **cluster-info**: Flags: --help, -h
- **config current-context**: Flags: --help, -h
- **config get-contexts**: Flags: --help, --no-headers, -h. Valued: --output, -o
- **config view**: Flags: --flatten, --help, --minify, --raw, -h. Valued: --output, -o
- **describe**: Flags: --all-namespaces, --help, --show-events, -A, -h. Valued: --namespace, --selector, -l, -n
- **diff**: Flags: --field-manager, --force-conflicts, --help, --prune, --server-side, -h. Valued: -f, --filename, -k, --kustomize, -l, --selector, --prune-allowlist. Positional args accepted
- **events**: Flags: --all-namespaces, --help, --watch, -A, -h, -w. Valued: --for, --namespace, --output, --types, -n, -o
- **explain**: Flags: --help, --recursive, -h. Valued: --api-version
- **get**: Flags: --all-namespaces, --help, --no-headers, --show-labels, --watch, -A, -h, -w. Valued: --field-selector, --label-selector, --namespace, --output, --selector, --sort-by, -l, -n, -o
- **logs**: Flags: --all-containers, --follow, --help, --previous, --timestamps, -f, -h, -p. Valued: --container, --namespace, --since, --tail, -c, -n
- **top node**: Flags: --help, --no-headers, -h. Valued: --selector, --sort-by, -l
- **top pod**: Flags: --all-namespaces, --containers, --help, --no-headers, -A, -h. Valued: --namespace, --selector, --sort-by, -l, -n
- **version**: Flags: --client, --help, --short, -h. Valued: --output, -o
- Allowed standalone flags: --help, --version, -V, -h

### `kubectx`
<p class="cmd-url"><a href="https://github.com/ahmetb/kubectx">https://github.com/ahmetb/kubectx</a></p>

- Allowed standalone flags: --current, --help, -c, -h
- Bare invocation allowed

### `kubens`
<p class="cmd-url"><a href="https://github.com/ahmetb/kubectx">https://github.com/ahmetb/kubectx</a></p>

- Allowed standalone flags: --current, --help, -c, -h
- Bare invocation allowed

### `kustomize`
<p class="cmd-url"><a href="https://kubectl.docs.kubernetes.io/references/kustomize/">https://kubectl.docs.kubernetes.io/references/kustomize/</a></p>

- **build**: Flags: --enable-alpha-plugins, --enable-exec, --enable-helm, --help, --load-restrictor, -h. Valued: --env, --helm-command, --mount, --output, -o
- **version**: Flags: --help, --short, -h
- Allowed standalone flags: --help, -h

### `minikube`
<p class="cmd-url"><a href="https://minikube.sigs.k8s.io/docs/">https://minikube.sigs.k8s.io/docs/</a></p>

- **addons list**: Flags: --help, --output, -h, -o. Valued: --profile, -p
- **ip**: Flags: --help, -h. Valued: --node, --profile, -n, -p
- **profile list**: Flags: --help, --light, --output, -h, -l, -o
- **service list**: Flags: --help, --namespace, -h. Valued: --profile, -p
- **status**: Flags: --format, --help, --output, -h, -o. Valued: --node, --profile, -n, -p
- **version**: Flags: --components, --help, --output, --short, -h, -o
- Allowed standalone flags: --help, --version, -h

### `orb`
<p class="cmd-url"><a href="https://docs.orbstack.dev/cli">https://docs.orbstack.dev/cli</a></p>

- **config get**: Flags: --help, -h
- **config show**: Flags: --help, -h
- **default**: Flags: --help, -h
- **doctor**: Flags: --help, -h
- **info**: Flags: --help, -h. Valued: --format, -f
- **list**: Flags: --help, --quiet, --running, -h, -q, -r. Valued: --format, -f
- **logs**: Flags: --all, --help, -a, -h
- **status**: Flags: --help, -h
- **update** (requires --check): Flags: --check, --help, -h
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -V, -h

### `orbctl`
<p class="cmd-url"><a href="https://docs.orbstack.dev/cli">https://docs.orbstack.dev/cli</a></p>

- **config get**: Flags: --help, -h
- **config show**: Flags: --help, -h
- **default**: Flags: --help, -h
- **doctor**: Flags: --help, -h
- **info**: Flags: --help, -h. Valued: --format, -f
- **list**: Flags: --help, --quiet, --running, -h, -q, -r. Valued: --format, -f
- **logs**: Flags: --all, --help, -a, -h
- **status**: Flags: --help, -h
- **update** (requires --check): Flags: --check, --help, -h
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -V, -h

### `podman`
<p class="cmd-url"><a href="https://docs.podman.io/en/latest/Commands.html">https://docs.podman.io/en/latest/Commands.html</a></p>

- **buildx --version**
- **buildx inspect**: Flags: --help, -h
- **buildx ls**: Flags: --help, -h
- **buildx version**: Flags: --help, -h
- **compose --version**
- **compose config**: Flags: --dry-run, --hash, --help, --images, --no-consistency, --no-interpolate, --no-normalize, --no-path-resolution, --profiles, --quiet, --resolve-image-digests, --services, --volumes, -h, -q. Valued: --format, --output, -o
- **compose images**: Flags: --help, -h
- **compose ls**: Flags: --help, -h
- **compose ps**: Flags: --all, --help, --no-trunc, --orphans, --quiet, --services, -a, -h, -q. Valued: --filter, --format, --status
- **compose top**: Flags: --help, -h
- **compose version**: Flags: --help, -h
- **container diff**: Flags: --help, -h
- **container inspect**: Flags: --help, --size, -h, -s. Valued: --format, --type, -f
- **container list**: Flags: --all, --help, --last, --latest, --no-trunc, --quiet, --size, -a, -h, -l, -n, -q, -s. Valued: --filter, --format, -f
- **container logs**: Flags: --details, --follow, --help, --timestamps, -f, -h, -t. Valued: --since, --tail, --until, -n
- **container ls**: Flags: --all, --help, --last, --latest, --no-trunc, --quiet, --size, -a, -h, -l, -n, -q, -s. Valued: --filter, --format, -f
- **container port**: Flags: --help, -h
- **container stats**: Flags: --all, --help, --no-stream, --no-trunc, -a, -h. Valued: --format
- **container top**: Flags: --help, -h
- **context inspect**: Flags: --help, --no-trunc, --quiet, -h, -q. Valued: --filter, --format, -f
- **context ls**: Flags: --help, --no-trunc, --quiet, -h, -q. Valued: --filter, --format, -f
- **context show**: Flags: --help, --no-trunc, --quiet, -h, -q. Valued: --filter, --format, -f
- **diff**: Flags: --help, -h
- **history**: Flags: --help, --human, --no-trunc, --quiet, -H, -h, -q. Valued: --format
- **image history**: Flags: --help, --human, --no-trunc, --quiet, -H, -h, -q. Valued: --format
- **image inspect**: Flags: --help, --size, -h, -s. Valued: --format, --type, -f
- **image list**: Flags: --all, --digests, --help, --no-trunc, --quiet, -a, -h, -q. Valued: --filter, --format, -f
- **image ls**: Flags: --all, --digests, --help, --no-trunc, --quiet, -a, -h, -q. Valued: --filter, --format, -f
- **images**: Flags: --all, --digests, --help, --no-trunc, --quiet, -a, -h, -q. Valued: --filter, --format, -f
- **info**: Flags: --help, -h. Valued: --format, -f
- **inspect**: Flags: --help, --size, -h, -s. Valued: --format, --type, -f
- **logs**: Flags: --details, --follow, --help, --timestamps, -f, -h, -t. Valued: --since, --tail, --until, -n
- **manifest inspect**: Flags: --help, --size, -h, -s. Valued: --format, --type, -f
- **network inspect**: Flags: --help, --no-trunc, --quiet, -h, -q. Valued: --filter, --format, -f
- **network ls**: Flags: --help, --no-trunc, --quiet, -h, -q. Valued: --filter, --format, -f
- **port**: Flags: --help, -h
- **ps**: Flags: --all, --help, --last, --latest, --no-trunc, --quiet, --size, -a, -h, -l, -n, -q, -s. Valued: --filter, --format, -f
- **stats**: Flags: --all, --help, --no-stream, --no-trunc, -a, -h. Valued: --format
- **system df**: Flags: --help, -h. Valued: --format, -f
- **system info**: Flags: --help, -h. Valued: --format, -f
- **top**: Flags: --help, -h
- **version**: Flags: --help, -h. Valued: --format, -f
- **volume inspect**: Flags: --help, --no-trunc, --quiet, -h, -q. Valued: --filter, --format, -f
- **volume ls**: Flags: --help, --no-trunc, --quiet, -h, -q. Valued: --filter, --format, -f
- Allowed standalone flags: --help, --version, -V, -h

### `qemu-img`
<p class="cmd-url"><a href="https://www.qemu.org/docs/master/tools/qemu-img.html">https://www.qemu.org/docs/master/tools/qemu-img.html</a></p>

- **check**: Flags: --force-share, --help, --image-opts, --quiet, -U, -h, -q. Valued: --cache, --format, --object, --output, -T, -f
- **compare**: Flags: --force-share, --help, --image-opts, --progress, --quiet, --strict, -U, -h, -p, -q, -s. Valued: --a-format, --b-format, --cache, --object, -F, -T, -f
- **info**: Flags: --backing-chain, --force-share, --help, --image-opts, --limits, -U, -h. Valued: --cache, --format, --object, --output, -f, -t
- **map**: Flags: --force-share, --help, --image-opts, -U, -h. Valued: --format, --max-length, --object, --output, --start-offset, -f, -l, -s
- **measure**: Flags: --force-share, --help, --image-opts, -U, -h. Valued: --format, --object, --output, --size, --snapshot, --target-format, -O, -f, -l, -o, -s
- **snapshot**: Flags: --force-share, --help, --image-opts, --list, --quiet, -U, -h, -l, -q. Valued: --format, --object, -f
- Allowed standalone flags: --help, --version, -V, -h

### `skopeo`
<p class="cmd-url"><a href="https://github.com/containers/skopeo">https://github.com/containers/skopeo</a></p>

- **inspect**: Flags: --config, --help, --no-creds, --raw, -h. Valued: --authfile, --cert-dir, --creds, --format, --tls-verify
- **list-tags**: Flags: --help, --no-creds, -h. Valued: --authfile, --cert-dir, --creds, --tls-verify
- **manifest-digest**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -h

### `stern`
<p class="cmd-url"><a href="https://github.com/stern/stern">https://github.com/stern/stern</a></p>

- Allowed standalone flags: --all-namespaces, --color, --diff-container, --ephemeral-containers, --help, --include-hidden, --init-containers, --no-follow, --only-log-lines, --timestamps, --version, -A, -h
- Allowed valued flags: --container, --container-state, --context, --exclude, --exclude-container, --exclude-pod, --highlight, --include, --kubeconfig, --max-log-requests, --namespace, --node, --output, --selector, --since, --tail, --template, --timezone, -c, -e, -l, -n, -o, -s, -t
- Hyphen-prefixed positional arguments accepted

