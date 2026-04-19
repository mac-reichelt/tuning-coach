# Security Policy

## Supported Versions

Pre-1.0 — only the latest release of each component (`sidecar`, `overlay`)
receives security fixes. After 1.0 we will adopt a longer support window.

## Reporting a Vulnerability

**Please do not open a public issue for sensitive vulnerabilities.**

Use [GitHub's private vulnerability reporting](https://github.com/mac-reichelt/tuning-coach/security/advisories/new)
instead.

You can expect:

1. Acknowledgment within a few days.
2. Triage and a planned fix or rejection (with reasoning) shortly after.
3. Credit in the release notes if you'd like, once the fix ships.

## Threat Model (current)

`tuning-coach` runs locally on a player's PC and:

- Listens on a UDP port (player-configured) for Forza telemetry — **not exposed
  to the public internet**.
- Serves a local WebSocket + HTTP API to the SimHub overlay — **bound to
  localhost by default**.
- Optionally calls an OpenAI-compatible LLM endpoint (player-configured) over
  HTTPS.
- Stores telemetry, recommendations, and player preferences in a local SQLite
  database — **not transmitted anywhere automatically**.

We treat as in-scope:

- Listener parsing bugs (buffer over-reads, panics, denial-of-service)
- WebSocket / HTTP authentication bypasses
- Path traversal in any file-handling code
- Secret leakage in logs or telemetry
- Dependency CVEs that affect a shipped binary

We treat as out-of-scope (for now):

- Local privilege escalation requiring already-local code execution
- Social engineering against the project maintainers
- Issues in third-party SimHub or Forza Motorsport software
