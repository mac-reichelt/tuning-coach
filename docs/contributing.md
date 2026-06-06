# Contributing to Tuning Coach

This project uses agent-driven review and strict CI. Before implementing any changes, identify which agents match the files you plan to touch.

## Agent Matrix

| Path | Agent(s) | Purpose |
|------|----------|---------|
| `sidecar/src/telemetry.rs`, `sidecar/src/forza_*.rs` | `telemetry-expert` | Packet schema is the source of truth; agent file must stay in sync with code |
| `sidecar/src/heuristics/**`, `sidecar/src/recommendations/**` | `race-engineer` + `telemetry-expert` | Tuning logic must reflect real-world practice + correct telemetry semantics |
| `sidecar/src/storage*.rs`, `sidecar/migrations/**` | `architect` | Schema migrations need ADR consideration |
| `docs/adr/**` (new files) | `architect` | New ADRs need review against existing decisions |
| `.github/workflows/**`, `.github/actions/**` | `devops-engineer` + `security-review` | CI/CD correctness + security (covered by devops-review.yml and security-review.yml) |
| Any file with auth, secrets, OIDC, crypto in name or context | `security-review` | OWASP / Zero Trust pass |
| New crates, new public modules, new sidecar tests | `qa-engineer` | Test strategy + coverage |
| `sidecar/web/**` (logic changes, not pure CSS) | `qa-engineer` | Web frontend test discipline (vitest) |

## Directory Structure

- `sidecar/` — Rust sidecar server and embedded web frontend
- `sidecar/web/` — Web overlay UI, dev/test tooling
- `simhub/` — SimHub dashboard bundle (`.djson`, `.metadata`, `.png`)
- `docs/` — Documentation, ADRs, guides

## Versioning and Releases

- The web frontend is embedded in the sidecar and versions/releases with it.
- The SimHub dashboard bundle in `simhub/` is not independently versioned; it ships as a release asset attached to the sidecar release.

## How to Contribute

1. **Fork and clone** the repository.
2. **Create a branch** for your changes.
3. **Consult the agent(s)** for any files you plan to edit (see matrix above).
4. **Write code and tests**. For web frontend logic (`sidecar/web/`), use `vitest` for JS tests.
5. **Document** any new features or changes in `README.md` or `docs/`.
6. **Open a PR**. Note which agents you consulted in your PR description.
7. **Follow CI feedback**. All PRs are checked for agent review, test coverage, and conventional commit messages.

## Conventional Commits

- Use [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) for all PR titles and commit messages.
- `feat:` for new features, `fix:` for bug fixes, `perf:` for performance improvements.
- Use `feat!:` and a `BREAKING CHANGE:` footer for breaking changes.

## License

SPDX: MIT — see [LICENSE](../LICENSE).
