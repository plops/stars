# Serialized Implementation Tasks (`task.md`)

Execute the following tasks sequentially. Each step includes explicit build, unit test, or integration validation commands.

---

### Task 1: Project Initialization & Dependency Configuration
- [x] Create Rust project structure with `Cargo.toml` in `/workspace/src/stars`.
- [x] Add required dependencies (`image`, `kamadak-exif`, `ratatui`, `crossterm`, `axum`, `tokio`, `tower-http`, `serde`, `serde_json`, `nalgebra`, `kiddo`, `sgp4`, `chrono`, `clap`, `anyhow`, `tracing`).
- [x] Verify project builds with `cargo check`.

*Validation Command*: `cargo check`

---

### Task 2: Core EXIF Parser & Synthetic Data Generator
- [x] Create `src/exif/mod.rs` to extract GPS coordinates, `DateTimeOriginal`, compass heading (`ImgDirection`), focal length, and camera model. Include fallbacks for images without EXIF.
- [x] Create `src/image_loader/mod.rs` with image decoding (JPEG, PNG) and a synthetic astrophotography frame generator capable of generating test images with ground horizon, background noise, known star positions, and EXIF tags.
- [x] Write unit tests for EXIF parsing and synthetic image generation in `src/exif/mod.rs` and `src/image_loader/mod.rs`.

*Validation Command*: `cargo test exif image_loader`

---

### Task 3: Star Detection & Centroiding Engine
- [x] Create `src/star_finder/mod.rs` implementing background noise estimation, adaptive thresholding, horizon/landscape masking, connected component clustering, and sub-pixel barycenter / 2D Gaussian centroid estimation `(x, y)`.
- [x] Write unit tests for star extraction precision and landscape masking in `src/star_finder/mod.rs`.

*Validation Command*: `cargo test star_finder`

---

### Task 4: Astrometry Engine & Bright Star Catalog
- [x] Create `src/astrometry/mod.rs` with embedded Hipparcos bright star catalog (mag $\le 6.5$), coordinate transforms (RA/Dec $\leftrightarrow$ Alt/Az $\leftrightarrow$ Pixel $x,y$), Local Sidereal Time solver, and geometric hashing KD-tree matching (`kiddo`).
- [x] Write unit tests for coordinate conversion and catalog searching in `src/astrometry/mod.rs`.

*Validation Command*: `cargo test astrometry`

---

### Task 5: EXIF Validation Engine
- [x] Create `src/validation/mod.rs` comparing expected celestial orientation from EXIF timestamp + GPS vs solved star positions to calculate time drift offset and compass heading deviation.
- [x] Write unit tests for EXIF timestamp drift detection in `src/validation/mod.rs`.

*Validation Command*: `cargo test validation`

---

### Task 6: Lens Aberration & Atmospheric Refraction Modeling
- [x] Create `src/aberration/mod.rs` implementing radial optical distortion ($k_1, k_2$), focal length estimation, PSF coma/astigmatism asymmetry calculation, and Bennett/Saastamoinen atmospheric refraction models.
- [x] Write unit tests for optical distortion fitting and atmospheric refraction in `src/aberration/mod.rs`.

*Validation Command*: `cargo test aberration`

---

### Task 7: Satellite Streak Detection & Tracking
- [x] Create `src/satellites/mod.rs` implementing Hough transform / RANSAC linear streak detection across single and multi-frame image sequences, and satellite trajectory matching using SGP4 TLE orbital propagation.
- [x] Write unit tests for streak detection and SGP4 propagation in `src/satellites/mod.rs`.

*Validation Command*: `cargo test satellites`

---

### Task 8: Interactive Terminal User Interface (Ratatui TUI)
- [x] Create `src/tui/mod.rs` building a multi-tab terminal UI in Ratatui with dashboard views:
  - Tab 1: System & Execution Overview
  - Tab 2: Star Field Canvas & Constellation Map
  - Tab 3: EXIF Metadata & Astrometric Validation
  - Tab 4: Lens Aberration & Refraction Heatmap
  - Tab 5: Satellite Streak Tracker
- [x] Verify TUI module compiles and runs in non-interactive mode.

*Validation Command*: `cargo check`

---

### Task 9: Axum Web Server & Interactive Browser UI
- [x] Create `src/web/mod.rs` exposing REST API endpoints (`/api/upload`, `/api/solve`, `/api/catalog`, `/api/sample`) and serving an interactive single-page web visualization application with responsive HTML5 canvas star viewer, aberration graphs, EXIF drift analysis, and dark mode astrophotography styling.
- [x] Write unit tests for Web API endpoints in `src/web/mod.rs`.

*Validation Command*: `cargo test web`

---

### Task 10: CLI Application & Integration Testing
- [x] Connect CLI parser (`src/main.rs`) supporting `--image`, `--sequence`, `--web`, `--tui`, `--port`, `--export-json`, `--sample`.
- [x] Create `tests/integration_tests.rs` verifying end-to-end processing of synthetic iPhone starfield images.
- [x] Run full test suite, lint check (`cargo clippy`), code formatting (`cargo fmt`).

*Validation Commands*:
- `cargo test`
- `cargo clippy -- -W clippy::all`
- `cargo fmt -- --check`

---

### Task 11: Git Commit & Walkthrough Documentation
- [x] Stage and commit all code changes with Conventional Commit format.
- [x] Generate walkthrough summary report in `plan/20260805_01_stars/walkthrough.md`.

*Validation Command*: `git log -n 5`
