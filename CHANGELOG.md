# Changelog

All notable changes to this project are documented here.

## [Unreleased]

### Added
- **CI:** Introduced `codeql-gate` workflow. This publishes a `codeql-gate` commit status required by branch protection. For Dependabot PRs, it accepts a neutral or success conclusion from CodeQL; for all other PRs, only success is accepted. This ensures dependency updates are not blocked by neutral CodeQL results.

### Changed
- **CI:** Updated `auto-merge.yml` to auto-merge PRs labeled `automerge` and all Dependabot PRs, including those not explicitly labeled.
- **CI:** Updated `pr-title.yml` to skip conventional commit title checks for Dependabot PRs, preventing spurious failures.

---
