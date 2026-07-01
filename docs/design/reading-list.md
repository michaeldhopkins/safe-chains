# Reading list — safety, capabilities, information flow

Status: **living document.** The theory the behavioral-taxonomy work stands on.
Add to it as we go; each entry gets a one-line "why it matters here." Group by
theme; keep newest-thinking (agents) at the end since it's the moving edge.

Convention: `Author, "Title" (year) — why it matters here.` Star (★) the handful
that are load-bearing for the current design.

## Foundational protection theory

- ★ Saltzer & Schroeder, "The Protection of Information in Computer Systems"
  (1975) — the eight principles (fail-safe defaults, least privilege, complete
  mediation, psychological acceptability). Our design vocabulary.
- Lampson, "Protection" (1971) — the access matrix; subjects × objects × rights.
- Anderson, *Security Engineering* (3rd ed., 2020) — the modern comprehensive
  reference; good for the book's citations.
- ★ CVSS v3.1 / v4 specification (FIRST) — metric decomposition (Attack Vector,
  Scope, Privileges Required, C/I/A impact). Our facets recapitulate these.
- MITRE ATT&CK — tactic/technique vocabulary (Persistence, Execution, Exfiltration)
  we can borrow term-for-term for the taxonomy and smell names.

## Capabilities & least authority

- Dennis & Van Horn, "Programming Semantics for Multiprogrammed Computations"
  (1966) — the origin of capabilities.
- ★ Hardy, "The Confused Deputy" (1988) — the through-line; every safe-chains
  vuln this year is an instance.
- ★ Miller, *Robust Composition: Towards a Unified Approach to Access Control and
  Concurrency Control* (PhD thesis, 2006) — object-capability model; ambient vs
  designated authority.
- Miller, Yee, Shapiro, "Capability Myths Demolished" (2003) — clears up
  capability misconceptions; ocap vs ACL.
- Shapiro, Smith, Farber, "EROS: a fast capability system" (1999); KeyKOS — ocap
  operating systems, proof the model is practical.

## Information flow & taint (current focus)

- ★ Denning, "A Lattice Model of Secure Information Flow" (1976) — THE foundation;
  labels + a lattice + allowed flows. Our confidentiality/integrity model *is*
  this.
- Denning & Denning, "Certification of Programs for Secure Information Flow"
  (1977) — static analysis of flows.
- Bell & LaPadula (1973) — confidentiality: no read-up, no write-down.
- ★ Biba, "Integrity Considerations for Secure Computer Systems" (1977) — the
  dual; no read-down, no write-up. This is the `curl | bash` axis and what we
  actually enforce.
- Myers & Liskov, "A Decentralized Model for Information Flow Control" (1997);
  Jif — DIFC: labels with owners + declassification. Our escape valves.
- ★ Sabelfeld & Myers, "Language-Based Information-Flow Security" (2003) — the
  survey; explicit vs implicit flows, declassification, what's tractable.
- Zeldovich et al., "Making Information Flow Explicit in HiStar" (2006); Krohn et
  al., "Information Flow Control for Standard OS Abstractions" — Flume (2007) —
  OS-level DIFC; the closest prior art to "IFC over processes/pipes."
- Schwartz, Avgerinos, Brumley, "All You Ever Wanted to Know About Dynamic Taint
  Analysis and Forward Symbolic Execution (but Might Have Been Afraid to Ask)"
  (2010) — the taint-tracking survey; sources, sinks, propagation, limits.
- Perl `perlsec` (taint mode) — a shipped, practical taint system for a
  scripting language; a useful concrete model.

## Sandboxing, isolation, syscall mediation

- ★ OpenBSD `pledge(2)` & `unveil(2)` man pages — a process declaring its own
  least authority; the ocap ideal for CLIs.
- Watson, Anderson, Laurie, Kennaway, "Capsicum: practical capabilities for UNIX"
  (2010) — capability mode for FreeBSD.
- Linux `capabilities(7)`, `seccomp` / seccomp-bpf, namespaces, **Landlock** LSM —
  the mechanisms behind the isolation strength ladder.
- Google gVisor (`runsc`); Kata Containers — the `vm`-strength rung.
- WebAssembly / WASI capability model (wasmtime docs) — deny-by-default,
  grant-by-`--dir`/`--tcplisten`; our isolation ceiling and design north star.
- ★ Garfinkel, "Traps and Pitfalls: Practical Problems in System Call
  Interposition Based Security Tools" (2003) — why interposition/mediation is
  hard (TOCTOU, indirect paths); cautionary for our own resolution passes.

## Supply chain & the shell

- ★ Thompson, "Reflections on Trusting Trust" (1984) — the supply-chain classic;
  provenance is unbounded.
- SLSA framework (slsa.dev) — supply-chain integrity levels; vocabulary for the
  `pinning`/`source` sub-facets.
- Package-manager references (for the ecosystem table): npm lifecycle-scripts &
  `--ignore-scripts` docs; PyPI wheel-vs-sdist / PEP 427; Go module authentication
  (`go.sum`, `GONOSUMDB`); The Cargo Book on build scripts; dpkg maintainer
  scripts.
- git CVE-2022-24765 / `safe.directory` — the confused-deputy-in-a-repo, the
  direct precedent for our trusted-config work.

## HCI & usable security (for the CLI-design database & the book)

- Norman, *The Design of Everyday Things* — affordances; make the safe path easy,
  the dangerous path explicit.
- Yee, "User Interaction Design for Secure Systems" (2002) — safe defaults,
  path of least resistance, visibility of authority.

## Agents / LLM security (the moving edge)

- ★ Willison, "The lethal trifecta for AI agents: private data + untrusted
  content + external communication" (2025) and his prompt-injection series —
  Denning's lattice restated for agents; the reason confidentiality flow is the
  sharp problem now.
- Greshake et al., "Not what you've signed up for: Compromising Real-World
  LLM-Integrated Applications with Indirect Prompt Injection" (2023) — untrusted
  content as a low-integrity source reaching a high-authority deputy.
- OWASP Top 10 for LLM Applications — working vocabulary for agent risks
  (prompt injection, insecure output handling, excessive agency).
