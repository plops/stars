# Task List: SIP Distortion & Plate Solving Enhancement

Serial task list for implementation. Each task should be completed, tested, and committed before proceeding to the next.

---

## Task 1: Externalize Star Catalog
- [x] Create `src/astrometry/catalog.rs` with `CatalogStar` struct and `load_catalog()` function
- [x] Create `data/bright_stars.csv` with Hipparcos bright stars (≤ mag 6.5, ~5000 stars)
  - Columns: name, ra_deg, dec_deg, vmag, spectral, parallax_mas, pmra_mas, pmdec_mas
- [x] Add `csv` dependency to `Cargo.toml` (latest version)
- [x] Modify `src/astrometry/mod.rs` to use external catalog instead of hardcoded list
- [x] Add `include_bytes!()` or runtime CSV loading with fallback to embedded catalog
- [x] **Test**: `cargo test test_catalog_loading` — verify ≥ 5000 stars loaded, magnitude range correct
- [x] **Validate**: `cargo clippy -- -W clippy::all && cargo fmt -- --check`
- [x] **Commit**: `feat(astrometry): externalize star catalog from hardcoded list to CSV`


## Task 2: Remove Hardcoded 45° Altitude Assumption
- [x] In `solve_plate()`, replace `let center_alt = 45.0` with iterative refinement
- [x] Implement `estimate_center_altitude()`:
  1. Start with initial guess from EXIF heading (if available)
  2. Try alt values [20°, 35°, 45°, 55°, 70°, 85°]
  3. For each, project catalog → pixel, count matches, pick best
  4. Refine around best with ±5° steps
- [x] Pass estimated altitude through `AstrometricSolution` struct
- [x] **Test**: `cargo test test_altitude_refinement` — synthetic image at alt=70° should solve correctly
- [x] **Validate**: `cargo clippy -- -W clippy::all && cargo fmt -- --check`
- [x] **Commit**: `fix(astrometry): replace hardcoded 45° altitude with iterative refinement`


## Task 3: Implement SIP Polynomial Distortion Model
- [ ] Create `src/astrometry/sip.rs` with:
  - `SipDistortion` struct holding A_p,q and B_p,q coefficients (up to order 4)
  - `apply_forward(u, v) -> (u', v')` — pixel → corrected pixel
  - `apply_inverse(u', v') -> (u, v)` — corrected → original pixel
  - `fit_from_residuals(matches: &[StarMatch], cx, cy) -> SipDistortion` — least squares fit
- [ ] Integrate into `solve_plate()`: after initial matching, fit SIP, re-project, re-match
- [ ] Add SIP coefficients to `AstrometricSolution` struct
- [ ] **Test**: `cargo test test_sip_forward_inverse` — round-trip error < 0.01 px
- [ ] **Test**: `cargo test test_sip_fit` — synthetic distorted image recovers k1, k2 within 10%
- [ ] **Validate**: `cargo clippy -- -W clippy::all && cargo fmt -- --check`
- [ ] **Commit**: `feat(astrometry): add SIP polynomial distortion model with forward/inverse transforms`

## Task 4: Atmospheric Refraction Integration
- [ ] Create `pub fn atmospheric_refraction_correction(alt_deg: f64) -> f64` in `aberration/mod.rs`
  - Bennett's formula returning correction in degrees
  - Handle edge cases: alt < 0° → return 0, alt = 0° → clamp to horizon refraction
- [ ] Integrate into `altaz_to_pixel()`: apply refraction correction to altitude before projection
- [ ] Option to enable/disable refraction correction
- [ ] **Test**: `cargo test test_refraction_at_horizon` — verify ~34 arcmin at 0°
- [ ] **Test**: `cargo test test_refraction_at_zenith` — verify ~0 arcmin at 90°
- [ ] **Validate**: `cargo clippy -- -W clippy::all && cargo fmt -- --check`
- [ ] **Commit**: `feat(aberration): integrate atmospheric refraction correction into projection pipeline`

## Task 5: Signed Residual Analysis in Validation
- [ ] Modify `StarMatch` to include signed residuals `dx_pixels` and `dy_pixels` (not just unsigned distance)
- [ ] Update `validate_exif()` to compute systematic heading error from mean signed X-residual
- [ ] Compute time drift from signed RA residual (Earth rotation direction)
- [ ] **Test**: `cargo test test_signed_residuals` — intentional 2° heading offset produces correct signed error
- [ ] **Validate**: `cargo clippy -- -W clippy::all && cargo fmt -- --check`
- [ ] **Commit**: `fix(validation): use signed directional residuals for heading and time drift detection`

## Task 6: Update Satellite TLE Data
- [ ] Replace hardcoded 2020-epoch TLEs with 2026-epoch data
- [ ] Add `fetch_tle()` stub function with fallback to embedded data
- [ ] Document that TLEs should be periodically updated
- [ ] **Test**: `cargo test test_satellite_streak_detection` — existing test passes with updated TLEs
- [ ] **Validate**: `cargo clippy -- -W clippy::all && cargo fmt -- --check`
- [ ] **Commit**: `fix(satellites): update stale 2020-epoch TLE data to current epoch`

## Task 7: Integration Tests for Real Images
- [ ] Add integration test for `/workspace/src/stars.jpg` with EXIF validation
- [ ] Add integration test for `/workspace/src/IMG_8550.jpg`
- [ ] Test that plate solving with new catalog finds ≥ 3 matches on real images
- [ ] Test SIP distortion fitting produces reasonable coefficients
- [ ] **Test**: `cargo test test_real_image` — both images process without panic
- [ ] **Validate**: `cargo clippy -- -W clippy::all && cargo fmt -- --check`
- [ ] **Commit**: `test(integration): add comprehensive real image pipeline tests`

## Task 8: Web UI Distortion Visualization
- [ ] Add distortion map overlay rendering to `web/mod.rs`
- [ ] Display SIP coefficients in Aberration tab
- [ ] Show atmospheric refraction correction curve
- [ ] Add interactive distortion grid visualization
- [ ] **Test**: `cargo test test_full_pipeline` — web pipeline test passes
- [ ] **Validate**: `cargo clippy -- -W clippy::all && cargo fmt -- --check`
- [ ] **Commit**: `feat(web): add SIP distortion map and atmospheric refraction visualization`

## Task 9: Documentation & Deps Update
- [ ] Update `deps.md` with any new dependencies (csv crate)
- [ ] Update `README.md` with SIP feature description
- [ ] Update `Cargo.toml` dependency versions to latest
- [ ] Run `cargo update` to refresh lockfile
- [ ] **Validate**: `cargo clippy -- -W clippy::all && cargo fmt -- --check`
- [ ] **Validate**: `cargo test` — all tests pass
- [ ] **Commit**: `docs: update dependencies, README, and deps.md for SIP feature`

## Task 10: Write Walkthrough Document
- [ ] Create `plan/20260807_01_sip/walkthrough.md`
- [ ] Document what was actually implemented vs planned
- [ ] List test results and coverage
- [ ] Document learnings and deviations
- [ ] List Docker programs needed
- [ ] List possible extensions
- [ ] **Commit**: `docs: add walkthrough document summarizing SIP implementation`
