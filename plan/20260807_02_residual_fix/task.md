# Task List: High Precision Plate Solving & Residual Reduction

Serial task log for implementing high precision plate solving and reducing residuals to < 5.0px RMSE.

---

- [ ] Task 1: Python Ground Truth Script & Baseline Generation <!-- id: 0 -->
  - Implement/run `python_prototype/validate_plate_solving.py` using `uv`.
  - Extract centroids and compute WCS solutions for `/workspace/src/stars.jpg` and `/workspace/src/IMG_8550.jpg`.
  - Save baseline diagnostic plots to `python_prototype/plots/`.
  - **Validation**: Verify ground-truth RA/Dec center coordinates and WCS matrix parameters.

- [ ] Task 2: Proper Motion Catalog Epoch Propagation <!-- id: 1 -->
  - Add proper motion propagation ($\mu_\alpha^*, \mu_\delta$) from Hipparcos epoch (1991.25) to observation timestamp UTC.
  - Apply in `src/astrometry/catalog.rs` or `radec_to_altaz_with_pm()`.
  - **Validation**: Unit test `test_proper_motion_propagation` in `catalog.rs`.

- [ ] Task 3: 2-Pass Matching & Affine Transformation Refinement <!-- id: 2 -->
  - Implement coarse match pass (radius = 25px).
  - Estimate 6-parameter 2D Affine Transformation matrix $(A, B, C, D, T_x, T_y)$ using `nalgebra::DMatrix` SVD/QR decomposition on coarse matched pairs.
  - Apply affine correction to projected catalog coordinates.
  - Perform second refined match pass with tight radius (12px).
  - **Validation**: Test affine solver on synthetic distorted grid (`test_affine_refinement`).

- [ ] Task 4: 3-Sigma Outlier Rejection & Robust Matching <!-- id: 3 -->
  - Compute residual statistics (mean $\mu$, std dev $\sigma$) of star matches.
  - Prune matches with residual $> \mu + 3\sigma$ or $> 15.0\text{px}$.
  - Refit SIP polynomial distortion model (`fit_from_residuals`) on pruned inliers only.
  - **Validation**: Test outlier rejection with noisy synthetic star field (`test_outlier_rejection`).

- [ ] Task 5: Plate Solve Quality Gates & Solution Criteria <!-- id: 4 -->
  - Update `is_solved` condition: require `matches.len() >= 4`, `rmse_pixels < 15.0`, and valid match ratio.
  - Return `solved: false` if RMSE exceeds quality gate threshold.
  - **Validation**: `cargo test` verifying all unit and integration tests pass.

- [ ] Task 6: Full Pipeline Testing & Validation on Real Images <!-- id: 5 -->
  - Process `/workspace/src/stars.jpg` and `/workspace/src/IMG_8550.jpg` through the updated Rust pipeline.
  - Compare RMSE and residual distribution before vs after fixes.
  - Verify `cargo clippy -- -W clippy::all` returns 0 warnings.
  - Verify `cargo fmt -- --check` passes.

- [ ] Task 7: Final Documentation & Walkthrough <!-- id: 6 -->
  - Create `plan/20260807_02_residual_fix/walkthrough.md` documenting implemented changes, test results, learnings, future expansions, and docker container dependencies.
  - Commit all changes using Conventional Commits.
