# Getting Started

Welcome to Tuning Coach! This guide walks you through installing and running the project, including setting up the SimHub overlay bundle.

## Prerequisites
- [Rust](https://rustup.rs/) 1.80 or newer
- SimHub (for overlay integration)

## Clone the Repository

```bash
git clone https://github.com/mac-reichelt/tuning-coach.git
cd tuning-coach
```

## Build and Run the Sidecar

```bash
cd sidecar
cargo build --release
./target/release/tuning-coach-sidecar
```

## Install the SimHub Overlay Bundle (v0.1.3+)

Tuning Coach provides a SimHub-importable overlay bundle for easy integration.

**To install:**
1. **Locate the overlay bundle:**
   - The bundle is provided in the `overlay/` directory as a `.zip` file (e.g., `tuning-coach-overlay-bundle.zip`).
   - If you built from source, run the overlay build script or download the latest release asset from [GitHub Releases](https://github.com/mac-reichelt/tuning-coach/releases).
2. **Open SimHub.**
3. **Go to Overlays > Import Overlay.**
4. **Select the `tuning-coach-overlay-bundle.zip` file.**
5. **Follow SimHub's prompts to finish the import.**
6. **Add the overlay to your layout.**

The overlay will connect to the running sidecar and display live telemetry and recommendations.

## Next Steps
- [Configuration](configuration.md)
- [API Reference](reference/api.md)
