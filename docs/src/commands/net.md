# Networking

### `curl`
<p class="cmd-url"><a href="https://curl.se/docs/manpage.html">https://curl.se/docs/manpage.html</a></p>

- Allowed standalone flags: --compressed, --fail, --globoff, --head, --insecure, --ipv4, --ipv6, --location, --no-buffer, --no-progress-meter, --show-error, --silent, --verbose, -4, -6, -I, -L, -N, -S, -f, -g, -k, -s, -v.
- Allowed valued flags: --connect-timeout, --max-time, --write-out, -m, -w.
- Allowed methods (-X/--request): GET, HEAD, OPTIONS.
- -H/--header allowed with safe headers (Accept, User-Agent, Authorization, Cookie, Cache-Control, Range, etc.).
- -o/--output and -O/--remote-name allowed (writes files).

### `dig`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/dig.1.html">https://man7.org/linux/man-pages/man1/dig.1.html</a></p>

- Allowed standalone flags: --help, --version, -4, -6, -V, -h, -m, -r, -u, -v
- Allowed valued flags: -b, -c, -f, -k, -p, -q, -t, -x, -y
- Bare invocation allowed

### `host`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/host.1.html">https://man7.org/linux/man-pages/man1/host.1.html</a></p>

- Allowed standalone flags: --help, --version, -4, -6, -C, -V, -a, -c, -d, -h, -l, -r, -s, -v
- Allowed valued flags: -D, -N, -R, -T, -W, -i, -m, -t

### `ifconfig`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man8/ifconfig.8.html">https://man7.org/linux/man-pages/man8/ifconfig.8.html</a></p>

- Allowed standalone flags: --help, --version, -L, -V, -a, -h, -l, -s, -v
- Bare invocation allowed

### `mdfind`
<p class="cmd-url"><a href="https://ss64.com/mac/mdfind.html">https://ss64.com/mac/mdfind.html</a></p>

- Allowed standalone flags: --help, --version, -0, -V, -count, -h, -interpret, -literal, -live
- Allowed valued flags: -attr, -name, -onlyin, -s

### `mtr`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man8/mtr.8.html">https://man7.org/linux/man-pages/man8/mtr.8.html</a></p>

- Allowed standalone flags: --help, --no-dns, --report, --report-wide, --show-ips, --version, -4, -6, -V, -b, -h, -n, -r, -w
- Allowed valued flags: --address, --count, --interval, --max-ttl, --psize, --report-cycles, --type, -I, -a, -c, -i, -m, -s

### `nc`
<p class="cmd-url"><a href="https://man.openbsd.org/nc.1">https://man.openbsd.org/nc.1</a></p>

- Requires -z. - Allowed standalone flags: --help, -h, -z, -v, -n, -u, -4, -6
- Allowed valued flags: -w

### `ncat`
<p class="cmd-url"><a href="https://nmap.org/ncat/">https://nmap.org/ncat/</a></p>

- Requires -z. - Allowed standalone flags: --help, -h, --version, -z, -v, -n, -u, -4, -6
- Allowed valued flags: -w, --wait

### `netstat`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man8/netstat.8.html">https://man7.org/linux/man-pages/man8/netstat.8.html</a></p>

- Allowed standalone flags: --all, --continuous, --extend, --groups, --help, --interfaces, --listening, --masquerade, --numeric, --numeric-hosts, --numeric-ports, --numeric-users, --program, --route, --statistics, --symbolic, --tcp, --timers, --udp, --unix, --verbose, --version, --wide, -A, -C, -L, -M, -N, -R, -S, -V, -W, -Z, -a, -b, -c, -d, -e, -f, -g, -h, -i, -l, -m, -n, -o, -p, -q, -r, -s, -t, -u, -v, -w, -x
- Allowed valued flags: -I
- Bare invocation allowed

### `nmap`
<p class="cmd-url"><a href="https://nmap.org/book/man.html">https://nmap.org/book/man.html</a></p>

- Allowed standalone flags: --help, --version, -h, -V, -sT, -sn, -sP, -sL, -sV, -Pn, -PE, -PP, -PM, -F, --open, --reason, --traceroute, -n, -R, -4, -6, -v, -vv, -vvv, -d, -d1, -d2, -d3, -d4, -d5, -d6, -d7, -d8, -d9, --packet-trace, --no-stylesheet, -T0, -T1, -T2, -T3, -T4, -T5, --system-dns, --version-light, --version-all
- Allowed valued flags: -p, --exclude-ports, --top-ports, --port-ratio, --max-retries, --host-timeout, --scan-delay, --max-scan-delay, --min-rate, --max-rate, --min-parallelism, --max-parallelism, --min-hostgroup, --max-hostgroup, --min-rtt-timeout, --max-rtt-timeout, --initial-rtt-timeout, --exclude, --dns-servers, -e, --source-port, -g, --ttl, --version-intensity

### `nslookup`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/nslookup.1.html">https://man7.org/linux/man-pages/man1/nslookup.1.html</a></p>

- Allowed: positional args, -debug, -nodebug, -d2, and valued options (-type=, -query=, -port=, -timeout=, -retry=, -class=, -domain=, -querytype=).

### `ping`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man8/ping.8.html">https://man7.org/linux/man-pages/man8/ping.8.html</a></p>

- Requires -c, --count. - Allowed standalone flags: -4, -6, -D, -O, -R, -a, -d, -n, -q, -v, --help, -h, --version, -V
- Allowed valued flags: --count, --deadline, --interface, --interval, --ttl, -I, -Q, -S, -W, -c, -i, -l, -s, -t, -w

### `route`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man8/route.8.html">https://man7.org/linux/man-pages/man8/route.8.html</a></p>

- Allowed subcommands: get, monitor, print, show
- Allowed flags: -4, -6, -n, -v
- Bare invocation allowed

### `ss`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man8/ss.8.html">https://man7.org/linux/man-pages/man8/ss.8.html</a></p>

- Allowed standalone flags: --all, --dccp, --extended, --family, --help, --info, --ipv4, --ipv6, --listening, --memory, --no-header, --numeric, --oneline, --options, --packet, --processes, --raw, --resolve, --sctp, --summary, --tcp, --tipc, --udp, --unix, --version, --vsock, -0, -4, -6, -E, -H, -O, -V, -a, -e, -h, -i, -l, -m, -n, -o, -p, -r, -s, -t, -u, -w, -x
- Allowed valued flags: --filter, --query, -A, -F, -f
- Bare invocation allowed

### `traceroute`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man8/traceroute.8.html">https://man7.org/linux/man-pages/man8/traceroute.8.html</a></p>

Aliases: `traceroute6`

- Allowed standalone flags: --help, --version, -4, -6, -F, -I, -T, -U, -V, -d, -e, -h, -n, -r, -v
- Allowed valued flags: --port, --queries, --sendwait, --wait, -f, -i, -m, -N, -p, -q, -s, -t, -w, -z

### `whois`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/whois.1.html">https://man7.org/linux/man-pages/man1/whois.1.html</a></p>

- Allowed standalone flags: --help, --version, -A, -B, -G, -H, -I, -K, -L, -M, -Q, -R, -S, -a, -b, -c, -d, -f, -g, -l, -m, -r, -x
- Allowed valued flags: -T, -V, -h, -i, -p, -s, -t

