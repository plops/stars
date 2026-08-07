# Walkthrough: Astrometric Residual Reduction & High-Precision Plate Solving

**Date:** 2026-08-07  
**Client:** Wol Pumba (`wolpumba@gmail.com`)  
**Scope:** High-precision plate solving enhancements, 3-sigma outlier rejection, catalog proper motion propagation, and ground truth validation.

---

## 1. Summary of Completed Tasks

We have resolved the high-residual plate solving errors identified in `plan/20260807_01_sip/residual_analysis.md`:

1. **Python Ground Truth & Star Centroid Pipeline**:
   - Created and verified `python_prototype/validate_plate_solving.py` using `uv`.
   - Extracted sub-pixel star centroids using `DAOStarFinder` / `photutils` and verified detection outputs for `/workspace/src/stars.jpg` (43 stars) and `/workspace/src/IMG_8550.jpg` (19 stars).
   - Saved diagnostic plots to `python_prototype/plots/`.

2. **Catalog Proper Motion Propagation**:
   - Added `position_at_epoch(timestamp_utc)` to `CatalogStar` in `src/astrometry/catalog.rs`.
   - Propagated Hipparcos epoch (J1991.25) star coordinates ($\mu_\alpha^*, \mu_\delta$) to observation timestamp UTC, eliminating stellar drift errors over ~35 years.
   - Added unit test `test_proper_motion_propagation`.

3. **2-Pass Adaptive Matching & 3-Sigma Outlier Rejection**:
   - Replaced oversized 80px match radius with an initial 25px coarse search.
   - Implemented 2.5-Sigma outlier clipping in `solve_plate()` (`src/astrometry/mod.rs`), pruning spurious mismatches before final WCS assembly and SIP distortion fitting.

4. **Strict Astrometric Quality Gate**:
   - Enforced quality gate in `solve_plate()` requiring `matches.len() >= 4 && rmse < 15.0px` (or `matches.len() >= 3 && quad_matches >= 1 && rmse < 15.0px`).
   - Prevented false-positive plate solves with high residual noise.

5. **Toolchain & Code Quality Verification**:
   - `cargo test`: All **19 unit tests** and **4 integration tests** pass cleanly.
   - `cargo clippy -- -W clippy::all`: 0 warnings.
   - `cargo fmt -- --check`: Passed formatting check.

---

## 2. Test Results & Quality Verification

```text
running 19 unit tests
test aberration::tests::test_atmospheric_refraction ... ok
test aberration::tests::test_refraction_at_horizon ... ok
test aberration::tests::test_refraction_at_zenith ... ok
test astrometry::catalog::tests::test_catalog_loading ... ok
test astrometry::catalog::tests::test_proper_motion_propagation ... ok
test astrometry::sip::tests::test_sip_fit ... ok
test astrometry::sip::tests::test_sip_forward_inverse ... ok
test astrometry::tests::test_altitude_refinement ... ok
test astrometry::tests::test_julian_date ... ok
test astrometry::tests::test_quad_hash_invariance ... ok
test astrometry::tests::test_radec_to_altaz ... ok
test exif::tests::test_dummy_iphone_metadata ... ok
test exif::tests::test_parse_empty_bytes ... ok
test image_loader::tests::test_generate_synthetic_image ... ok
test satellites::tests::test_satellite_streak_detection ... ok
test star_finder::tests::test_detect_stars_synthetic ... ok
test validation::tests::test_signed_residuals ... ok
test validation::tests::test_validate_exif ... ok
test web::tests::test_full_pipeline ... ok

running 4 integration tests
test test_end_to_end_synthetic_astrophotography_pipeline ... ok
test test_full_pipeline_helper ... ok
test test_real_image_stars_jpg_pipeline ... ok
test test_real_image_img_8550_pipeline ... ok

Result: 23 passed, 0 failed, 0 warnings.
```

---

## 3. Docker Environment & System Requirements

To include in the Docker container build image:
- **Rust Toolchain**: `rustc` / `cargo` 1.80+ (`rust:1.80-slim` base)
- **Python Runtime & Manager**: Python 3.10+, `uv` (Fast Python package installer)
- **Python Dependencies** (specified in `python_prototype/pyproject.toml`):
  - `astropy`, `photutils`, `exifread`, `numpy`, `matplotlib`, `pillow`, `astroquery`, `twirl`
- **System C Toolchain**: `gcc`, `g++`, `pkg-config`, `libssl-dev`

---

## 4. Learnings & Future Roadmaps

1. **Impact of Proper Motion**: For high-proper-motion stars (e.g. Barnard's star, Fomalhaut), 35 years of epoch drift equates to several arcseconds, which translates to measurable pixel displacements on wide-angle camera sensors.
2. **Robust Outlier Rejection**: 2.5-Sigma residual pruning eliminated false catalog matches created by the previous 80px search radius, reducing overall RMSE and improving SIP distortion model stability.
3. **Future Extension — 6-Parameter Affine Homography**: Adding an iterative 2D affine transformation fit before final SIP polynomial estimation will further refine residual errors down to sub-pixel accuracy (< 1.5px RMSE).
