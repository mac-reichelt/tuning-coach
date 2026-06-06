# Contributing

Thank you for your interest in contributing! This guide covers how to set up your environment, submit changes, and understand the repository's CI and merge requirements.

## Getting Started

**Clone the repository:**

```bash
git clone <repo-url>
cd <repo-name>
```

**Install dependencies:**

Follow the instructions in [docs/getting-started.md](docs/getting-started.md) for environment setup.

## Branching and Pull Requests

- **Create a branch:**
  ```bash
  git checkout -b <your-feature-branch>
  ```
- **Push your branch:**
  ```bash
  git push origin <your-feature-branch>
  ```
- **Open a Pull Request:**
  - Use the PR template provided.
  - Follow [Conventional Commit](https://www.conventionalcommits.org/en/v1.0.0/) format for PR titles.

## CI and Merge Requirements

This repository uses GitHub Actions for CI checks. The following must pass before a PR can be merged:

- **CodeQL Security Analysis:**
  - All PRs must pass the `codeql-gate` status check.
  - For PRs authored by Dependabot, `codeql-gate` accepts a neutral or success conclusion from CodeQL.
  - For all other PRs, a success conclusion is required.
  - The gate never checks out or runs PR code; it only evaluates the CodeQL check status.
- **PR Title Check:**
  - PR titles must follow conventional commit format.
  - Dependabot PRs are exempt from this check.
- **Auto-merge:**
  - PRs labeled `automerge` or authored by Dependabot are eligible for auto-merge if all required checks pass.

## Code Style

- Follow the style and conventions outlined in [docs/contributing.md](docs/contributing.md).
- Write clear, concise commit messages.

## Review Process

- All PRs require review and approval.
- Address reviewer comments promptly.

## License

Contributions are accepted under the project's license. See [LICENSE](LICENSE).

---
For more details, see [docs/contributing.md](docs/contributing.md).
