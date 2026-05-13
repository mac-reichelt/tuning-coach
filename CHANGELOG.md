## [Unreleased]

### Changed
- Overlay is now served directly from the sidecar HTTP endpoint (`127.0.0.1:7778`); SimHub overlay points to this address.
- Removed `overlay/manifest.json` and legacy overlay bundle import instructions.
- Updated docs and README to reflect new overlay serving mechanism.

### Migration
- If you previously used the SimHub overlay bundle or `manifest.json`, switch to using `tuning-coach.djson` and `tuning-coach.djson.metadata` pointing to `http://127.0.0.1:7778/`.
- No external static file server required; run only `tuning-coach-sidecar`.
