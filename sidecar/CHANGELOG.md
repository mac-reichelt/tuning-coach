# Changelog

## [0.1.5](https://github.com/mac-reichelt/tuning-coach/compare/sidecar-v0.1.4...sidecar-v0.1.5) (2026-05-10)


### Features

* **overlay:** add SimHub-importable overlay bundle and install docs ([#87](https://github.com/mac-reichelt/tuning-coach/issues/87)) ([c6e66bf](https://github.com/mac-reichelt/tuning-coach/commit/c6e66bf38ee2c3c1a2db7c5d5c246a04210e4c96))

## [0.1.4](https://github.com/mac-reichelt/tuning-coach/compare/sidecar-v0.1.3...sidecar-v0.1.4) (2026-04-26)


### Features

* **sidecar:** add storage accessors for recommendations table + car_setups read ([#84](https://github.com/mac-reichelt/tuning-coach/issues/84)) ([1ad509b](https://github.com/mac-reichelt/tuning-coach/commit/1ad509b4e036b09c728e0da853f5188e1ca5eeca))

## [0.1.3](https://github.com/mac-reichelt/tuning-coach/compare/sidecar-v0.1.2...sidecar-v0.1.3) (2026-04-26)


### Features

* **sidecar+docs:** adr-0003 recommendation WS payload contract + typed Rust structs + stub endpoint ([#83](https://github.com/mac-reichelt/tuning-coach/issues/83)) ([c84bd28](https://github.com/mac-reichelt/tuning-coach/commit/c84bd2857720ae9daa4f5d32b010bdbe28a57f54))

## [0.1.2](https://github.com/mac-reichelt/tuning-coach/compare/sidecar-v0.1.1...sidecar-v0.1.2) (2026-04-26)


### Bug Fixes

* **sidecar:** support FM2023 331-byte Dash packets; fix Windows WSAEMSGSIZE (os error 10040) ([#48](https://github.com/mac-reichelt/tuning-coach/issues/48)) ([53c0e12](https://github.com/mac-reichelt/tuning-coach/commit/53c0e12d73cb123e1ba9a4f4f977d47af614a85b))

## [0.1.1](https://github.com/mac-reichelt/tuning-coach/compare/sidecar-v0.1.0...sidecar-v0.1.1) (2026-04-21)


### Features

* initial repo scaffold ([2f8ab4a](https://github.com/mac-reichelt/tuning-coach/commit/2f8ab4ab3dcbef7ff80754c556c14e1de05f8bf7))
* **sidecar:** add ADR-aligned SQLite schema and migration runner for persisted session data ([#34](https://github.com/mac-reichelt/tuning-coach/issues/34)) ([3c25519](https://github.com/mac-reichelt/tuning-coach/commit/3c25519cdc8ab5a1674581ea7ceb0d0a516d8b50))
* **sidecar:** add Forza UDP Dash/Sled parser and telemetry ingest channel ([#33](https://github.com/mac-reichelt/tuning-coach/issues/33)) ([e4b2b35](https://github.com/mac-reichelt/tuning-coach/commit/e4b2b35cbcd8da516a18a672b61b66e0a53acdad))
* **sidecar:** add manual override hotkey REST endpoints for lap validity ([#42](https://github.com/mac-reichelt/tuning-coach/issues/42)) ([5afb0c8](https://github.com/mac-reichelt/tuning-coach/commit/5afb0c8a3f1c796c4865972cf4193b46a6857554))
* **sidecar:** add typed debounced session state machine with SQLite lifecycle persistence ([#37](https://github.com/mac-reichelt/tuning-coach/issues/37)) ([7c63004](https://github.com/mac-reichelt/tuning-coach/commit/7c6300420224ac5b2caf7421e46d60069e1af37b))
* **sidecar:** add v1 WebSocket overlay stream contract (telemetry + recommendation fan-out) ([#35](https://github.com/mac-reichelt/tuning-coach/issues/35)) ([5ec6dfc](https://github.com/mac-reichelt/tuning-coach/commit/5ec6dfc63490f8d408c923e65737ec6103dffb99))
* **sidecar:** detect dirty laps from telemetry heuristics ([#39](https://github.com/mac-reichelt/tuning-coach/issues/39)) ([cb30baf](https://github.com/mac-reichelt/tuning-coach/commit/cb30baff1d39e6bfb82de6ced1c4d0d87e79c06c))
* **sidecar:** detect lap rewind and session reset from telemetry ([#38](https://github.com/mac-reichelt/tuning-coach/issues/38)) ([d2d38a8](https://github.com/mac-reichelt/tuning-coach/commit/d2d38a845a630de3084f22f1fb418e8be049003d))
* **sidecar:** detect pit-stop entry/exit from telemetry with hysteresis and lap invalidation ([#40](https://github.com/mac-reichelt/tuning-coach/issues/40)) ([7295bd4](https://github.com/mac-reichelt/tuning-coach/commit/7295bd491a24862db0e8535edc2df6bfec743957))
* **sidecar:** scaffold tokio/axum sidecar runtime with config, logging, health, and WS skeleton ([#15](https://github.com/mac-reichelt/tuning-coach/issues/15)) ([8aadfd7](https://github.com/mac-reichelt/tuning-coach/commit/8aadfd73d4c454b6848a40a6bf3699c73da747ec))
