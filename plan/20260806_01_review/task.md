# Implementation & Testing Tasks

This document lists sequential tasks to address all code review findings and implement required software enhancements for `stars`.

---

- [x] **Task 1: Star Catalog Expansion & Geometric Quad Hashing Solver (`src/astrometry/mod.rs`)**
  - Expand `get_bright_star_catalog()` with 45+ stars across major constellations.
  - Implement 4D Geometric Quad Hashing $((u_1, v_1, u_2, v_2))$ scale and rotation invariant hash generation.
  - Index catalog quads in `kiddo::KdTree<f64, 4>`.
  - Remove hardcoded fake fallback matches from `solve_plate()`.
  - Add unit tests: `test_quad_hash_invariance` and `test_expanded_catalog_solve`.

- [x] **Task 2: Connected Component Star Detection & Centroiding (`src/star_finder/mod.rs`)**
  - Implement region growing / connected component labeling to cluster adjacent star pixels above noise floor.
  - Compute sub-pixel centroids, FWHM, and SNR across complete connected component extents.
  - Add unit test: `test_detect_stars_connected_components`.

- [x] **Task 3: Authentic RGB Chromatic Aberration & Lens Distortion (`src/aberration/mod.rs`)**
  - Implement RGB channel separation in centroiding to compute Red vs Blue channel centroid shifts near image boundaries.
  - Calculate radial distortion polynomial coefficients $k_1, k_2$ via least-squares residuals.
  - Remove hardcoded synthetic multipliers ($0.05, 0.8, 1.5$).
  - Add unit test: `test_rgb_chromatic_aberration`.

- [x] **Task 4: Multi-Satellite TLE Registry & Dynamic SGP4 Orbit Propagation (`src/satellites/mod.rs`)**
  - Add TLE registry for ISS, Hubble (HST), Tiangong (CSS), and Starlink satellites.
  - Propagate orbits dynamically to image `timestamp_utc` using `sgp4`.
  - Match detected linear streaks against projected satellite ground tracks with confidence scoring.
  - Add unit test: `test_multi_satellite_sgp4_propagation`.

- [x] **Task 5: EXIF Orientation & Celestial Timestamp Drift Validation (`src/exif/mod.rs`, `src/validation/mod.rs`)**
  - Parse EXIF orientation tags (tag `0x0112`) and apply pixel coordinate transformations.
  - Refine EXIF timestamp drift validation based on solved RA celestial delta ($15^\circ/\text{hr}$).
  - Add unit test: `test_exif_orientation_and_validation`.

- [x] **Task 6: Web Server API & Ratatui TUI Dashboard Upgrades (`src/web/mod.rs`, `src/tui/mod.rs`)**
  - Add API query parameters for star detection sigma thresholds and overlay layer toggles.
  - Upgrade web interface styling and interactive visual overlays (vector fields, satellite tracks, catalog labels).
  - Enhance Ratatui TUI dashboard screens.

- [x] **Task 7: Code Verification, Linting, & Unit/Integration Testing**
  - Run `cargo check`.
  - Run `cargo test` (ensuring 100% pass rate for unit and integration tests).
  - Run `cargo clippy -- -W clippy::all`.
  - Run `cargo fmt -- --check`.

- [x] **Task 8: Conventional Git Commits & Walkthrough Generation**
  - Format git commit messages using Conventional Commit format with comprehensive descriptions.
  - Write `plan/20260806_01_review/walkthrough.md` summarizing changes, test results, learnings, and Docker container package recommendations.
