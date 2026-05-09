# Containers

### `buildah`
<p class="cmd-url"><a href="https://github.com/containers/buildah/tree/main/docs">https://github.com/containers/buildah/tree/main/docs</a></p>

- **containers**: Flags: --all, --help, --json, --no-trunc, --noheading, --notruncate, --quiet, -a, -h, -q. Valued: --filter, --format, -f
- **help**: Positional args accepted
- **images**: Flags: --all, --digests, --help, --history, --json, --no-trunc, --noheading, --quiet, -a, -h, -n, -q. Valued: --filter, --format, -f. Positional args accepted
- **info**: Flags: --debug, --help, -h. Valued: --format
- **inspect**: Flags: --help, -h. Valued: --format, --type, -f, -t. Positional args accepted
- **version**: Flags: --help, -h. Valued: --format, --json
- Allowed standalone flags: --help, --version, -h, -v

### `buildctl`
<p class="cmd-url"><a href="https://github.com/moby/buildkit">https://github.com/moby/buildkit</a></p>

- **completion**: Flags: --help, -h. Positional args accepted
- **debug histories**: Flags: --help, -h
- **debug info**: Flags: --help, -h
- **debug workers**: Flags: --filter, --help, --verbose, -h, -v
- **du**: Flags: --filter, --help, --verbose, -h, -v
- **help**: Positional args accepted
- Allowed standalone flags: --help, --version, -h, -v

### `copa`
<p class="cmd-url"><a href="https://project-copacetic.github.io/copacetic/website/">https://project-copacetic.github.io/copacetic/website/</a></p>

- **help**: Positional args accepted
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -h, -v

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

### `ctr`
<p class="cmd-url"><a href="https://github.com/containerd/containerd/blob/main/docs/getting-started.md">https://github.com/containerd/containerd/blob/main/docs/getting-started.md</a></p>

- **help**: Positional args accepted
- **info**: Flags: --help, -h
- **namespaces list**: Flags: --help, --quiet, -h, -q
- **namespaces ls**: Flags: --help, --quiet, -h, -q
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -h, -v

### `dapr`
<p class="cmd-url"><a href="https://docs.dapr.io/reference/cli/">https://docs.dapr.io/reference/cli/</a></p>

- **completion**: Flags: --help, -h. Positional args accepted
- **components**: Flags: --all-namespaces, --help, --kubernetes, -A, -h, -k. Valued: --name, --namespace, --output, -n, -o
- **configurations**: Flags: --all-namespaces, --help, --kubernetes, -A, -h, -k. Valued: --name, --namespace, --output, -n, -o
- **help**: Positional args accepted
- **list**: Flags: --all-namespaces, --help, -A, -h. Valued: --kubernetes, --namespace, --output, -k, -n, -o
- **status**: Flags: --help, --kubernetes, -h, -k. Valued: --output, -o
- **version**: Flags: --help, -h. Valued: --output, -o
- Allowed standalone flags: --help, --version, -h, -v

### `dive`
<p class="cmd-url"><a href="https://github.com/wagoodman/dive">https://github.com/wagoodman/dive</a></p>

- Allowed standalone flags: --ci, --help, --source, --version, -h, -v
- Allowed valued flags: --ci-config, --config, --highestPercentage, --highestUserWastedPercent, --lowestEfficiency, --source

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

### `docker-buildx`
<p class="cmd-url"><a href="https://docs.docker.com/buildx/working-with-buildx/">https://docs.docker.com/buildx/working-with-buildx/</a></p>

- **completion**: Flags: --help, -h. Positional args accepted
- **du**: Flags: --help, --verbose, -h, -v. Valued: --builder, --filter
- **help**: Positional args accepted
- **inspect**: Flags: --bootstrap, --help, -h. Valued: --builder. Positional args accepted
- **ls**: Flags: --help, --no-trunc, -h. Valued: --format
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -h, -v

### `docker-compose`
<p class="cmd-url"><a href="https://docs.docker.com/compose/reference/">https://docs.docker.com/compose/reference/</a></p>

- **completion**: Flags: --help, -h. Positional args accepted
- **config**: Flags: --format, --help, --no-consistency, --no-interpolate, --no-normalize, --no-path-resolution, --profiles, --quiet, --resolve-image-digests, --services, --volumes, -h, -q. Valued: --format, --hash, --output, --profile, -o, -p
- **events**: Flags: --help, --json, -h. Positional args accepted
- **help**: Positional args accepted
- **images**: Flags: --help, --quiet, -h, -q. Valued: --format. Positional args accepted
- **logs**: Flags: --follow, --help, --no-color, --no-log-prefix, --timestamps, -f, -h, -t. Valued: --since, --tail, --until. Positional args accepted
- **port**: Flags: --help, -h. Valued: --index, --protocol. Positional args accepted
- **ps**: Flags: --all, --dry-run, --help, --no-trunc, --quiet, --services, -a, -h, -q. Valued: --filter, --format, --status. Positional args accepted
- **top**: Flags: --help, -h. Positional args accepted
- **version**: Flags: --format, --help, --short, -f, -h. Valued: --format
- Allowed standalone flags: --help, --version, -h, -v

### `flux`
<p class="cmd-url"><a href="https://fluxcd.io/flux/cmd/flux/">https://fluxcd.io/flux/cmd/flux/</a></p>

- **check**: Flags: --help, --pre, -h. Valued: --components, --components-extra, --kustomization, --source
- **completion**: Flags: --help, -h. Positional args accepted
- **help**: Positional args accepted
- **version**: Flags: --client, --help, -h. Valued: --output, -o
- Allowed standalone flags: --help, --version, -h, -v

### `hadolint`
<p class="cmd-url"><a href="https://github.com/hadolint/hadolint">https://github.com/hadolint/hadolint</a></p>

- Allowed standalone flags: --config, --disable-ignore-pragma, --failure-threshold, --format, --help, --ignore, --no-color, --no-fail, --require-label, --strict-labels, --trusted-registry, --verbose, --version, -V, -c, -h, -t, -v
- Allowed valued flags: --config, --failure-threshold, --format, --ignore, --require-label, --trusted-registry, -c, -f, -t
- Hyphen-prefixed positional arguments accepted

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

### `istioctl`
<p class="cmd-url"><a href="https://istio.io/latest/docs/reference/commands/istioctl/">https://istio.io/latest/docs/reference/commands/istioctl/</a></p>

- **analyze**: Flags: --all-namespaces, --color, --failure-threshold, --help, --ignore-unknown, --list-analyzers, --meshConfigFile, --no-color, --output, --output-threshold, --recursive, --revision, --suppress, --timeout, --use-kube, --verbose, -A, -L, -R, -S, -h, -o. Valued: --failure-threshold, --meshConfigFile, --namespace, --output, --output-threshold, --remote-contexts, --revision, --suppress, --timeout, --use-kube, --verbose, -S, -n, -o
- **completion**: Flags: --help, -h. Positional args accepted
- **help**: Positional args accepted
- **proxy-config all**: Flags: --help, -h
- **proxy-config bootstrap**: Flags: --help, -h
- **proxy-config cluster**: Flags: --help, -h
- **proxy-config endpoint**: Flags: --help, -h
- **proxy-config listener**: Flags: --help, -h
- **proxy-config route**: Flags: --help, -h
- **proxy-config secret**: Flags: --help, -h
- **proxy-status**: Flags: --file, --help, --multi-xds, --xds-via-agents, -f, -h. Valued: --file, --multi-xds, --output, --revision, -f, -o. Positional args accepted
- **validate**: Flags: --help, --no-validate, --output, --quiet, --referential, -f, -h, -o, -q. Valued: --filename, --output, -f, -o. Positional args accepted
- **version**: Flags: --help, --remote, --revision, --short, -h, -s. Valued: --filename, --output, --revision, -f, -o
- Allowed standalone flags: --help, --version, -h, -v

### `kapp`
<p class="cmd-url"><a href="https://carvel.dev/kapp/">https://carvel.dev/kapp/</a></p>

- **completion**: Flags: --help, -h. Positional args accepted
- **help**: Positional args accepted
- **list**: Flags: --all-namespaces, --help, --namespace, -A, -h, -n. Valued: --namespace, -n
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -h, -v

### `kind`
<p class="cmd-url"><a href="https://kind.sigs.k8s.io/">https://kind.sigs.k8s.io/</a></p>

- **get clusters**: Flags: --help, -h
- **get kubeconfig**: Flags: --help, --internal, -h. Valued: --name
- **get nodes**: Flags: --help, -h. Valued: --name
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -h

### `kn`
<p class="cmd-url"><a href="https://knative.dev/docs/client/configure-kn/">https://knative.dev/docs/client/configure-kn/</a></p>

- **completion**: Flags: --help, -h. Positional args accepted
- **help**: Positional args accepted
- **revision describe**: Flags: --help, -h
- **revision list**: Flags: --help, -h
- **service describe**: Flags: --help, --no-headers, --print-details, --verbose, -h, -v. Valued: --namespace, --output, -n, -o
- **service list**: Flags: --all-namespaces, --help, --no-headers, -A, -h. Valued: --namespace, --output, -n, -o
- **version**: Flags: --help, --version, -h
- Allowed standalone flags: --help, --version, -h, -v

### `kpt`
<p class="cmd-url"><a href="https://kpt.dev/reference/cli/">https://kpt.dev/reference/cli/</a></p>

- **completion**: Flags: --help, -h. Positional args accepted
- **help**: Positional args accepted
- **pkg cat**: Flags: --help, -h. Valued: --annotate, --exclude-non-local, --include-local, --strip-comments, --style
- **pkg diff**: Flags: --help, -h. Valued: --diff-tool, --diff-tool-opts, --diff-type
- **pkg tree**: Flags: --help, -h
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -h, -v

### `krew`
<p class="cmd-url"><a href="https://krew.sigs.k8s.io/">https://krew.sigs.k8s.io/</a></p>

- **completion**: Flags: --help, -h. Positional args accepted
- **help**: Positional args accepted
- **index list**: Flags: --help, -h
- **info**: Flags: --help, -h. Positional args accepted
- **list**: Flags: --help, -h
- **search**: Flags: --help, -h. Positional args accepted
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -h, -v

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

### `linkerd`
<p class="cmd-url"><a href="https://linkerd.io/2/reference/cli/">https://linkerd.io/2/reference/cli/</a></p>

- **check**: Flags: --allow-mismatched-policy-controller, --cli-version-override, --config, --cni-namespace, --crds, --deletion-pending, --enable-pprof, --expected-version, --help, --ipv6, --linkerd-cni-enabled, --multicluster, --no-policy, --no-tls-policy, --output, --pre, --proxy, --retry-deadline, --short, --wait, -h, -o. Valued: --cni-namespace, --config, --expected-version, --namespace, --output, --retry-deadline, --wait, -n, -o
- **completion**: Flags: --help, -h. Positional args accepted
- **help**: Positional args accepted
- **version**: Flags: --client, --help, --proxy, --short, -h. Valued: --namespace
- Allowed standalone flags: --help, --version, -h, -v

### `minikube`
<p class="cmd-url"><a href="https://minikube.sigs.k8s.io/docs/">https://minikube.sigs.k8s.io/docs/</a></p>

- **addons list**: Flags: --help, --output, -h, -o. Valued: --profile, -p
- **ip**: Flags: --help, -h. Valued: --node, --profile, -n, -p
- **profile list**: Flags: --help, --light, --output, -h, -l, -o
- **service list**: Flags: --help, --namespace, -h. Valued: --profile, -p
- **status**: Flags: --format, --help, --output, -h, -o. Valued: --node, --profile, -n, -p
- **version**: Flags: --components, --help, --output, --short, -h, -o
- Allowed standalone flags: --help, --version, -h

### `nerdctl`
<p class="cmd-url"><a href="https://github.com/containerd/nerdctl">https://github.com/containerd/nerdctl</a></p>

- **completion**: Flags: --help, -h. Positional args accepted
- **events**: Flags: --help, -h. Valued: --filter, --format, --since, --until, -f
- **help**: Positional args accepted
- **image history**: Flags: --help, --human, --no-trunc, --quiet, -H, -h, -q. Valued: --format
- **image inspect**: Flags: --help, --mode, -h. Valued: --format, --mode, -f
- **image ls**: Flags: --all, --digests, --help, --names, --no-trunc, --quiet, -a, -h, -q. Valued: --filter, --format, -f
- **images**: Flags: --all, --digests, --help, --names, --no-trunc, --quiet, -a, -h, -q. Valued: --filter, --format, -f. Positional args accepted
- **info**: Flags: --debug, --help, --mode, -h. Valued: --format, --mode, -f
- **inspect**: Flags: --help, --mode, --size, -h, -s. Valued: --format, --mode, --type, -f. Positional args accepted
- **logs**: Flags: --details, --follow, --help, --timestamps, -f, -h, -t. Valued: --since, --tail, --until, -n. Positional args accepted
- **network inspect**: Flags: --help, -h. Valued: --format, --mode, -f
- **network ls**: Flags: --help, --no-trunc, --quiet, -h, -q. Valued: --filter, --format, -f
- **port**: Flags: --help, -h. Positional args accepted
- **ps**: Flags: --all, --help, --latest, --no-trunc, --quiet, --size, -a, -h, -l, -n, -q, -s. Valued: --filter, --format, --last, -f
- **stats**: Flags: --all, --help, --no-stream, --no-trunc, -a, -h. Valued: --format. Positional args accepted
- **system info**: Flags: --help, -h. Valued: --format, --mode, -f
- **top**: Flags: --help, -h. Positional args accepted
- **version**: Flags: --help, --format, -f, -h. Valued: --format, -f
- **volume inspect**: Flags: --help, -h. Valued: --format, -f
- **volume ls**: Flags: --help, --quiet, --size, -h, -q. Valued: --filter, --format, -f
- Allowed standalone flags: --help, --version, -h, -v

### `oras`
<p class="cmd-url"><a href="https://oras.land/docs/category/cli-reference">https://oras.land/docs/category/cli-reference</a></p>

- **blob fetch**: Flags: --help, --insecure, --plain-http, --verbose, -h, -v. Valued: --ca-file, --cert-file, --config, --header, --key-file, --media-type, --output, --password, --registry-config, --username, -o, -p, -u
- **completion**: Flags: --help, -h. Positional args accepted
- **discover**: Flags: --help, --insecure, --no-color, --no-tty, --plain-http, -h. Valued: --artifact-type, --ca-file, --cert-file, --config, --distribution-spec, --format, --header, --key-file, --output, --password, --password-stdin, --registry-config, --username, --verbose, -c, -d, -o, -p, -u, -v. Positional args accepted
- **help**: Positional args accepted
- **manifest fetch**: Flags: --help, --insecure, --plain-http, --verbose, -h, -v. Valued: --ca-file, --cert-file, --config, --header, --key-file, --media-type, --output, --password, --platform, --registry-config, --username, -o, -p, -u
- **manifest fetch-config**: Flags: --help, -h. Valued: --ca-file, --cert-file, --config, --header, --key-file, --media-type, --output, --password, --platform, --registry-config, --username, -o, -p, -u
- **repo ls**: Flags: --help, -h. Valued: --last
- **repo tags**: Flags: --help, -h. Valued: --last
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -h, -v

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

### `tkn`
<p class="cmd-url"><a href="https://tekton.dev/docs/cli/">https://tekton.dev/docs/cli/</a></p>

- **completion**: Flags: --help, -h. Positional args accepted
- **help**: Positional args accepted
- **pipeline describe**: Flags: --help, --last, -h, -L. Valued: --namespace, --output, -n, -o
- **pipeline list**: Flags: --all-namespaces, --help, --no-headers, -A, -h. Valued: --namespace, --output, -n, -o
- **pipeline logs**: Flags: --all, --follow, --help, --last, --prefix, --step, --task, --timestamps, -a, -f, -h. Valued: --namespace, --limit, -n
- **version**: Flags: --client, --component, --help, -h
- Allowed standalone flags: --help, --version, -h, -v

### `velero`
<p class="cmd-url"><a href="https://velero.io/docs/">https://velero.io/docs/</a></p>

- **completion**: Flags: --help, -h. Positional args accepted
- **help**: Positional args accepted
- **version**: Flags: --client-only, --help, --timeout, -h. Valued: --namespace, --timeout, -n
- Allowed standalone flags: --help, --version, -h, -v

### `ytt`
<p class="cmd-url"><a href="https://carvel.dev/ytt/">https://carvel.dev/ytt/</a></p>

- Allowed standalone flags: --debug, --dangerous-allow-all-symlink-destinations, --data-values-inspect, --data-values-schema-inspect, --debug-skip-default-files, --help, --ignore-unknown-comments, --implied-template-files, --quiet, --strict, --symlink-allow, --symlink-allow-all, --type-strict, --version, -h, -q, -v
- Allowed valued flags: --data-value, --data-value-file, --data-value-yaml, --data-values-env, --data-values-env-yaml, --data-values-file, --debug-output, --file-mark, --filter, --ignored-paths, --output, --output-files, --output-directory, --output-multi-files, --symlink-allow, -f, -o
- Bare invocation allowed

