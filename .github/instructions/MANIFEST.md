# Vendored Copilot Instructions

These `*.instructions.md` files are vendored copies from
[github/awesome-copilot](https://github.com/github/awesome-copilot).

GitHub Copilot reads any `.github/instructions/*.instructions.md` file whose
`applyTo:` frontmatter glob matches the current file path. Updates land here
manually; re-vendor from upstream when the originals change.

| File | Source | applyTo |
|---|---|---|
| `security-and-owasp.instructions.md` | awesome-copilot/instructions/ | `**` |
| `ai-prompt-engineering-safety-best-practices.instructions.md` | awesome-copilot/instructions/ | `*` |
| `containerization-docker-best-practices.instructions.md` | awesome-copilot/instructions/ | `**/Dockerfile,**/compose*.yml,...` |
