# Contributing to Tuning Coach

Welcome! Follow these steps to contribute code, docs, or tests.

## Getting Started

1. **Fork and clone**
   ```bash
   git clone https://github.com/<your-org>/tuning-coach.git
   cd tuning-coach
   ```
2. **Create a branch**
   ```bash
   git checkout -b <feature-or-bugfix>
   ```
3. **Identify agent routing**
   - Before editing, consult the [agent routing matrix](.github/copilot-instructions.md#agent-routing) to determine which agent(s) must review your changes.
   - Note consulted agents in your PR description: `Consulted: <agent-name> per routing matrix`.

## Making Changes

- **Code:**
  - Follow Rust and JS/TS style guides.
  - Add tests for new public functions/modules.
  - For tuning logic, ensure heuristics match real-world engineering practice.
- **Docs:**
  - Update relevant pages in `docs/`.
  - Use active voice, second person, and scannable formatting.

## Automated Review Workflows

Every PR triggers four automated agent review workflows:

| Workflow         | Scope                                                        |
|------------------|-------------------------------------------------------------|
| security-review  | Security-sensitive files, workflows, actions, shell scripts  |
| qa-review        | Source changes without accompanying tests                    |
| telemetry-review | Telemetry schema and expert logic                            |
| heuristics-review| Tuning logic and race engineering heuristics                 |

These checks must pass for your PR to merge. Out-of-scope PRs are auto-approved.

## Opening a PR

1. **Push your branch**
   ```bash
   git push origin <branch>
   ```
2. **Open a pull request**
   - Fill out the PR template.
   - List consulted agents per the routing matrix.
   - Link related issues.

## Running Tests

- **Rust:**
  ```bash
  cargo test
  ```
- **Overlay (JS/TS):**
  ```bash
  npm test
  ```

## Docs

- [Getting Started](docs/getting-started.md)
- [API Reference](docs/reference/api.md)
- [Lap Validity](docs/reference/lap-validity.md)

## License
MIT — see [LICENSE](LICENSE).
