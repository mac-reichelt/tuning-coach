# Contributing

Welcome! Follow these steps to contribute to Tuning Coach.

## Getting Started

1. **Fork and clone**
   ```bash
   git clone https://github.com/<your-org>/tuning-coach.git
   cd tuning-coach
   ```
2. **Install Rust** ([rustup.rs](https://rustup.rs))
3. **Build**
   ```bash
   cargo build --release
   ```
4. **Run tests**
   ```bash
   cargo test
   ```

## Agent Routing and Review Matrix

Before editing, check which agent(s) must review your changes. See [docs/contributing.md](docs/contributing.md) for the full routing matrix.

- **Security-sensitive files**: security-review agent
- **CI/CD workflows**: devops-engineer + security-review agents
- **Telemetry schema**: telemetry-expert agent
- **Heuristics/recommendations**: race-engineer + telemetry-expert agents
- **New crates/modules/tests**: qa-engineer agent

Automated review workflows enforce this matrix:
- `.github/workflows/security-review.yml`
- `.github/workflows/qa-review.yml`
- `.github/workflows/telemetry-review.yml`
- `.github/workflows/heuristics-review.yml`

Out-of-scope PRs are auto-approved; in-scope PRs require agent verdicts.

## Making a PR

1. **Create a branch**
   ```bash
   git checkout -b <feature-name>
   ```
2. **Make your changes**
3. **Run tests**
   ```bash
   cargo test
   ```
4. **Document**
   - Update relevant docs in `docs/` and `README.md`.
   - If you change agent-reviewed files, note consulted agents in your PR description.
5. **Push and open a PR**
   ```bash
   git push origin <feature-name>
   ```

## PR Review Process

- Automated agent reviews run for every PR.
- If your PR changes files in the agent routing matrix, the relevant agent(s) will review.
- Address any agent feedback before merging.

## Coding Standards

- Use active voice and present tense in docs.
- Add or update tests for new public functions/modules.
- Link new concepts to their definitions.

## License

MIT — see [LICENSE](LICENSE).
