# ADR 0004: Relocate the served frontend into the sidecar; reserve `simhub/` for the dashboard bundle

- Status: proposed
- Date: 2026-06-05
- Deciders: @mac-reichelt
- Supersedes: —
- Extends: —

## Context

The repo's top-level `overlay/` directory has, since the project began, been
treated as "the SimHub overlay." Investigation of `overlay/tuning-coach.djson`
shows this is no longer accurate. The dashboard's only content is a
`WebPageItem` with `"StartAddress": "http://127.0.0.1:7778/"`. SimHub loads
nothing from disk for the UI — it embeds a Chromium browser pointed at the
sidecar's HTTP origin. The sidecar already serves and embeds the web app via
`rust_embed` (`sidecar/src/overlay.rs`, `#[folder = "../overlay/"]`, routes
`/`, `/src/{*path}`, `/styles/{*path}`), and the page only works when served
by the sidecar (its WS client uses `ws://${location.host}/ws` and the dyno
panel POSTs to the origin-relative `/api/v1/dyno/reset`).

Therefore the HTML/CSS/JS in `overlay/` is functionally the **sidecar's
frontend**, not a standalone SimHub artifact. The only files a SimHub user
actually imports are the dashboard bundle: `tuning-coach.djson`,
`tuning-coach.djson.metadata`, and the preview PNGs. The current layout
conflates three different things in one directory — the SimHub import bundle,
the served frontend, and JS dev/test tooling (`package.json`, `vitest.config.js`,
`*.test.js`, `dev/`) — and props up a release-please `node` package
(`overlay`, manifest `0.1.4`) whose `package.json` has no `version` field and
whose output (`overlay-v*` tag, tarball of `overlay/`) ships nothing a user
runs, because the frontend is already embedded in the sidecar binary.

This ADR locks the target layout, the versioning model, the embed/serve
contract, and the CI/release surface before the mechanical move is made, so the
refactor lands as one clean, reviewable change.

## Decision

We will split `overlay/` into two single-purpose locations:

1. **`simhub/`** (renamed from `overlay/`) holds *strictly* the SimHub import
   bundle: `tuning-coach.djson`, `tuning-coach.djson.metadata`, and the preview
   PNGs. Nothing else lives here.
2. **`sidecar/web/`** holds the served frontend and its dev/test tooling,
   preserving the existing relative tree: `index.html`, `src/*.js`,
   `styles/overlay.css`, `dev/`, `*.test.js`, `vitest.config.js`,
   `package.json`, `package-lock.json`, and a frozen `CHANGELOG.md`.

The frontend **versions and releases with the sidecar**. We will remove the
`overlay` package from `release-please-config.json` and the `overlay` key from
`release-please-manifest.json`; `package.json` remains only as vitest dev
tooling. The SimHub dashboard bundle is not independently versioned — it ships
as a release asset attached to the sidecar release.

`sidecar/src/overlay.rs` will point `rust_embed` at `#[folder = "web/"]` with an
explicit `#[include]` allowlist (`index.html`, `src/*`, `styles/*`) so that
dev/test files and any `node_modules` are never embedded into the binary. The
HTTP routes are unchanged.

## Consequences

### Positive
- `simhub/` now means exactly what its name says — the SimHub import bundle —
  eliminating the "overlay = the UI" confusion that motivated this ADR.
- The served frontend lives next to the server that embeds it; a frontend change
  is correctly a sidecar change and bumps the sidecar version.
- One release artifact and one version stream (`vX.Y.Z`, the `tuning-coach` package). The broken
  `overlay` node package (no `version` field) is retired.
- The `#[include]` allowlist guarantees `package-lock.json`/`node_modules`/tests
  can never be baked into the release binary, regardless of future dev files.
- Routes, WS URL, dyno fetch, the `*.test.js` vitest glob, and the `./src/*.js`
  test imports all keep working unchanged because the relative tree is preserved.

### Negative / trade-offs accepted
- `overlay-vX.Y.Z` tags stop being produced. Existing overlay tags/releases stay
  immutable, but tooling or links that assumed an ongoing overlay tag stream must
  be updated.
- The dashboard bundle loses an independent release cadence. Accepted: it changes
  rarely and is meaningless without a compatible sidecar. A future ADR can
  re-introduce a `simhub` release package if a separate cadence is ever needed.
- A short-lived window exists where new frontend files land in `overlay/` (via
  in-flight work) and are then `git mv`d; rename history is preserved.

### Neutral
- `sidecar/src/overlay.rs` keeps its filename; it remains the embed/serve adapter.
- `telemetry-review`/`heuristics-review` workflows key on `sidecar/src/...` and are
  unaffected; only `qa-review` (which keyed on `overlay/*`) moves to `sidecar/web/*`.

## Alternatives considered

### Keep the directory named `overlay/`
Perpetuates the exact confusion this ADR exists to remove — the folder would hold
the SimHub *dashboard*, not the overlay UI. The rename is the point.

### Name the dashboard folder `dashboard/`
Generic and readable, but doesn't convey that these are SimHub-proprietary
`.djson` import files; `simhub/` names the consumer tool and the artifacts precisely.

### Put the frontend in `sidecar/assets/` or `sidecar/frontend/`
`assets/` implies passive static files but the tree includes tests, a dev tool,
and npm tooling; `frontend/` reads like a separately deployed app rather than
something embedded into the binary. `sidecar/web/` is concise and matches "the
web stuff the server serves."

### Keep the `overlay` release-please node package at the new location
The package is already broken (no `version` field), and its release output ships
nothing a user runs because the frontend is embedded in the binary. Versioning
with the sidecar reflects reality.

### Use `#[exclude]` instead of `#[include]` on the embed
A denylist is a footgun — one missed pattern (e.g. a future `node_modules`) ships
test/tooling files inside the release binary. An allowlist embeds only what is served.

## References

- ADR-0002 — WS API contract (origin-relative `/ws` consumed by the frontend)
- ADR-0003 — recommendation payload (frontend renderer is the consumer)
- `sidecar/src/overlay.rs` — embed + serve adapter (`#[folder]`, routes)
- `.github/release-please-config.json`, `.github/release-please-manifest.json`
- `.github/workflows/{ci,qa-review,release-build}.yml`
- `simhub/tuning-coach.djson` — `WebPageItem` → `http://127.0.0.1:7778/`