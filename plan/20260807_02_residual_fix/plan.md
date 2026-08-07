# Implementation Plan: High Precision Plate Solving & Residual Reduction

**Goal**: Fix systematic astrometric residual errors identified in `plan/20260807_01_sip/residual_analysis.md` (mean residual 33.1px → < 5.0px RMSE) and establish python-based ground truth validation for test images `/workspace/src/stars.jpg` and `/workspace/src/IMG_8550.jpg`.

---

## 1. Context & Essential File List

An independent AI agent should inspect the following key files before working on this task:

| File | Description & Relevance |
|------|-------------------------|
| `src/astrometry/mod.rs` | Main plate solving engine (`solve_plate`, `estimate_center_altitude`, `altaz_to_pixel`). Contains matching radius (currently 80px), projection logic, and solution assembly. |
| `src/astrometry/sip.rs` | SIP polynomial distortion fitting (`fit_from_residuals`, `apply_forward`, `apply_inverse`). Contains least-squares solver for higher-order optical aberrations. |
| `src/astrometry/catalog.rs` | Star catalog loader (`CatalogStar` struct with proper motion `pmra_mas`, `pmdec_mas`, parallax `parallax_mas`, and CSV deserializer). |
| `src/aberration/mod.rs` | Lens aberration analysis (radial distortion $k_1, k_2$, coma, astigmatism) and Bennett's atmospheric refraction. |
| `src/validation/mod.rs` | EXIF validation report with directional signed residuals (`dx_pixels`, `dy_pixels`) for compass heading and time drift calculation. |
| `python_prototype/validate_plate_solving.py` | Python ground-truth script using `astropy`, `photutils`, and `twirl` for reference star detection and WCS solving. |
| `plan/20260807_01_sip/residual_analysis.md` | Empirical analysis of the 5 root causes of 30+ pixel residuals. |

---

## 2. Commit Message & Workflow Guidelines

All commits **MUST** follow Conventional Commits format with a comprehensive description body:

```text
<type>(<scope>): <short summary in imperative present tense>

<detailed multi-line description explaining:
- What problem was solved
- Mathematical or algorithmic rationale
- Specific changes made in files
- Impact on test metrics and residual RMSE>
```

Types: `feat`, `fix`, `refactor`, `test`, `docs`, `perf`.

---

## 3. Cargo Workflow & Dependency Rules

Follow `/workspace/src/rs-summarizer/.kiro/skills/cargo-workflow/SKILL.md`:
- Linting: `cargo clippy -- -W clippy::all` (0 warnings required).
- Formatting: `cargo fmt -- --check`.
- Build & Test: `cargo test` (all unit & integration tests green).
- If introducing dependencies: Update to latest versions, log in `deps.md` with GitHub organization link, and query deepwiki MCP for usage patterns if applicable.

---

## 4. Root Causes & Rust Solution Architecture

### Root Cause 1: Oversized Match Radius (80px → Adaptive 15px)
- **Problem**: 80px threshold (~1.3° sky angle) causes false star associations.
- **Solution**: Reduce initial coarse match radius to 25px, then perform a second pass after affine alignment with a tight 10px-15px match threshold.

### Root Cause 2: Single-Pass Forward Matching (No Affine Refinement)
- **Problem**: Projection model relies purely on initial altitude/azimuth estimate without solving for focal length scale, camera roll angle, or translation offsets.
- **Solution**: Implement a 2-pass iterative solver:
  1. Coarse match pass (25px radius).
  2. Compute 6-parameter 2D Affine Transformation $(a, b, c, d, tx, ty)$ via Least Squares / SVD on matched pairs.
  3. Apply affine correction to projected coordinates.
  4. Refine match pass (12px radius).

### Root Cause 3: No Outlier Rejection (Sigma Clipping / RANSAC)
- **Problem**: Spurious or misidentified star matches skew the final solution and inflate RMSE.
- **Solution**: Implement 3-Sigma Clipping on match residuals during affine fitting to purge outliers before computing the final WCS solution and SIP polynomial model.

### Root Cause 4: Missing Proper Motion Epoch Propagation
- **Problem**: Catalog star positions are given at J2000 / Hipparcos epoch (1991.25). Without proper motion propagation ($\mu_{\alpha}^*, \mu_{\delta}$), fast-moving nearby stars drift by several arcseconds over ~35 years.
- **Solution**: Propagate catalog coordinates $\alpha(t) = \alpha_0 + \frac{\text{pmra}}{\cos(\delta_0)} \Delta t$ and $\delta(t) = \delta_0 + \text{pmdec} \cdot \Delta t$ in `radec_to_altaz` or catalog loading.

### Root Cause 5: Quality Gate & Validation (RMSE Threshold)
- **Problem**: `is_solved` returns `true` even with 38px RMSE if $\ge 3$ matches exist.
- **Solution**: Enforce `is_solved = matches.len() >= 4 && rmse < 15.0 && inlier_ratio >= 0.5`.

---

## 5. Python Ground Truth & Verification Pipeline

- Run `python_prototype/validate_plate_solving.py` using `uv run python`.
- Extract star centroids (`DAOStarFinder` / `photutils`) and compute WCS solutions for `/workspace/src/stars.jpg` and `/workspace/src/IMG_8550.jpg`.
- Save detection and residual diagnostic plots to `python_prototype/plots/`.
- Use Python WCS results as benchmark targets for the Rust pipeline.

---

## 6. Implementation Strategy & Deliverables

1. **Step 1**: Build Python ground-truth script & execute verification on real images.
2. **Step 2**: Implement Proper Motion propagation in `src/astrometry/catalog.rs` & `mod.rs`.
3. **Step 3**: Implement 2-pass affine transformation & 3-sigma outlier rejection in `src/astrometry/mod.rs`.
4. **Step 4**: Reduce match threshold to adaptive 15px & enforce quality gate (`rmse < 15.0`).
5. **Step 5**: Run full test suite (`cargo test`, `cargo clippy`, `cargo fmt`).
6. **Step 6**: Generate python plots comparing Rust vs Python residuals.
7. **Step 7**: Commit all changes with Conventional Commit format and write `walkthrough.md`.
