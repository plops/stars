# Implementation Plan: SIP Distortion & Plate Solving Validation

**Author**: AI Agent (on behalf of wol pumba, wolpumba@gmail.com)  
**Date**: 2026-08-07  
**Scope**: Code review, Python validation prototype, SIP distortion modeling, enhanced plate solving

---

## 1. Executive Summary

This plan covers three main work streams:
1. **Software Review** — Comprehensive code review of the existing Rust `stars` codebase
2. **Python Validation Prototype** — Minimal Python prototype to validate plate solving and atmospheric/distortion algorithms using real astronomical libraries (`twirl`, `photutils`, `astropy`)
3. **SIP Distortion Implementation Plan** — Roadmap for adding proper SIP (Simple Imaging Polynomial) distortion support to the Rust solver

---

## 2. Code Review Findings

### 2.1 Critical Issues

| # | Module | Issue | Severity |
|---|--------|-------|----------|
| 1 | `astrometry` | Hardcoded bright star catalog with only ~50 stars (mag ≤ 3.8 effective) — far below claimed 6.5 mag limit. Real catalogs like Hipparcos have 9110 stars to mag 6.5. | **Critical** |
| 2 | `astrometry` | Fixed `center_alt = 45.0°` assumption — plate solver always assumes camera is pointed at 45° altitude, severely limiting accuracy for any other pointing angle | **Critical** |
| 3 | `satellites` | Hardcoded 2020-epoch TLE data — TLEs expire within weeks, these are 6+ years stale. SGP4 propagation over such intervals is meaningless. | **High** |
| 4 | `astrometry` | No SIP or any polynomial distortion correction — gnomonic projection assumes perfect pinhole camera, iPhone lenses have significant barrel distortion | **High** |
| 5 | `validation` | Heading error derived from RMSE magnitude rather than signed directional residuals — conflates random error with systematic drift | **Medium** |
| 6 | `aberration` | Fallback distortion coefficients `(k1=0.0001, k2=0.00001)` when fit fails — these should be NaN or Option<f64> | **Medium** |

### 2.2 Architectural Strengths
- Clean module separation with well-defined interfaces
- BFS connected-component star detection is sound
- 4D Quad Hash implementation is mathematically correct
- EXIF parsing handles both EXIF colon-format and ISO hyphen-format dates
- Sobel-based horizon detection is a practical approach
- RGB chromatic aberration measurement using per-channel centroids is novel

### 2.3 Hardcoded Values Requiring Configuration

| Value | Location | Current | Recommended |
|-------|----------|---------|-------------|
| Match distance | `astrometry/mod.rs:592` | 80px | Configurable, scale-dependent |
| Horizon detection mean/variance | `star_finder/mod.rs:249` | mean>32, var>110 | Should adapt to image statistics |
| RANSAC seed | `satellites/mod.rs:98` | `42` | Configurable or time-seeded |
| Streak threshold | `satellites/mod.rs:82` | `140u8` | Should use background statistics |
| Quality score weights | `aberration/mod.rs:110-114` | Fixed multipliers | Tunable via config |

---

## 3. Star Catalog Analysis

### 3.1 Current Rust Catalog
- **Size**: 50 stars hardcoded in `get_bright_star_catalog()`
- **Magnitude range**: -1.46 (Sirius) to 3.77 (Sualocin)
- **No parallax data**: Coordinates are fixed J2000 epoch
- **Coverage**: Northern hemisphere biased, sparse southern sky

### 3.2 Python Prototype Catalog (Gaia via twirl)
- **Gaia DR3**: 1.468 billion sources with positions, parallaxes, proper motions
- **Typical cone search**: ~500-5000 stars per FOV depending on magnitude limit
- **Parallax available**: Yes, with microarcsecond precision
- **Storage**: Dynamic query, no local storage needed (or ~200GB for full local index)

### 3.3 Parallax Relevance for iPhone Astrophotography

**Conclusion: Parallax is negligible for this application.**

The maximum stellar parallax is 0.768 arcseconds (Proxima Centauri). For typical iPhone astrophotography:
- **iPhone pixel scale**: ~15-20 arcsec/pixel (26mm equiv, 4032px width, ~65° FOV)
- **Maximum parallax shift**: 0.768" ≈ 0.04 pixels
- **Typical star parallax**: <0.1" ≈ <0.005 pixels

This is far below sub-pixel detection precision. Proper motion (≫ parallax for nearby stars over years) is more relevant for catalog epoch corrections but still sub-pixel for J2000→2026.

---

## 4. SIP Distortion Feature — Implementation Approach

### 4.1 What is SIP?
The Simple Imaging Polynomial convention extends FITS WCS to model optical distortions via polynomial coefficients:

$$f(u,v) = \sum_{p,q} A_{p,q} \cdot u^p \cdot v^q$$
$$g(u,v) = \sum_{p,q} B_{p,q} \cdot u^p \cdot v^q$$

where $(u,v)$ are intermediate pixel coordinates relative to CRPIX, and the distortion order is typically 2-4 for consumer lenses.

### 4.2 Proposed Architecture

```
src/
├── astrometry/
│   ├── mod.rs           # Existing plate solver (enhanced)
│   ├── catalog.rs       # NEW: Externalized star catalog loading
│   ├── sip.rs           # NEW: SIP polynomial distortion model
│   └── projection.rs    # NEW: Gnomonic projection with distortion
├── aberration/
│   └── mod.rs           # Enhanced with SIP-informed quality metrics
```

### 4.3 Key Requirements

| Requirement | Description | Priority |
|------------|-------------|----------|
| R1 | SIP polynomial evaluation (forward: pixel→sky, inverse: sky→pixel) | **Must** |
| R2 | Polynomial coefficient fitting from star match residuals | **Must** |
| R3 | Atmospheric refraction correction integrated into projection | **Must** |
| R4 | Externalized star catalog (CSV/binary, ≥500 stars mag ≤ 6.5) | **Must** |
| R5 | Iterative center altitude refinement (remove 45° hardcode) | **Must** |
| R6 | Diagnostic output: distortion map, residual plot data | **Should** |
| R7 | SIP coefficient export in FITS WCS-compatible format | **Should** |
| R8 | iPhone lens profile presets (barrel distortion k1/k2 per model) | **Could** |
| R9 | Multi-epoch catalog support (proper motion correction) | **Could** |
| R10 | Live TLE fetching from CelesTrak API | **Could** |

### 4.4 Additional Requirements Not in Original Prompt

1. **Iterative altitude refinement**: The current hardcoded 45° pointing assumption makes the solver unreliable. An iterative least-squares refinement of (center_alt, center_az) is essential.
2. **Signed residual analysis**: The validation module needs signed (directional) residuals, not just unsigned Euclidean distances, to detect systematic drift.
3. **Catalog epoch correction**: J2000 coordinates should be precessed to the observation epoch (proper motion × Δt years).
4. **Web UI distortion visualization**: Interactive distortion map overlay in the web dashboard.
5. **Benchmark tests**: Performance regression tests for the solver with varying star counts and FOV sizes.

---

## 5. File Reference for Implementation Agent

Each file listed should be read by an implementing agent to build context:

| File | Purpose |
|------|---------|
| `src/astrometry/mod.rs` | Core plate solver with QuadHash, catalog, projection functions. ~774 lines. Contains `solve_plate()`, `altaz_to_pixel()`, coordinate transforms. |
| `src/star_finder/mod.rs` | BFS star detection, horizon masking, background estimation. ~315 lines. Produces `DetectedStar` structs consumed by plate solver. |
| `src/aberration/mod.rs` | Radial distortion fitting (k1/k2), coma/astigmatism analysis, Bennett refraction. ~222 lines. |
| `src/exif/mod.rs` | EXIF parsing using `kamadak-exif`. ~234 lines. Extracts GPS, timestamp, focal length, orientation. |
| `src/validation/mod.rs` | Cross-validates EXIF metadata against astrometric solution. ~102 lines. |
| `src/satellites/mod.rs` | RANSAC streak detection, SGP4 orbit propagation. ~341 lines. Contains hardcoded TLEs. |
| `src/image_loader/mod.rs` | Image loading and synthetic image generation. ~302 lines. |
| `src/main.rs` | CLI entry point with clap argument parsing. ~145 lines. |
| `src/lib.rs` | Module re-exports. ~10 lines. |
| `src/web/mod.rs` | Axum web server with embedded HTML/CSS/JS. ~1056 lines. |
| `src/tui/mod.rs` | Ratatui terminal dashboard. ~286 lines. |
| `tests/integration_tests.rs` | End-to-end pipeline tests. ~114 lines. |
| `Cargo.toml` | Dependency manifest. |
| `deps.md` | Dependency documentation with GitHub organizations. |
| `plan/20260805_01_stars/` | Initial implementation plan and research. |
| `plan/20260806_01_review/` | First code review findings and fixes. |
| `plan/20260807_01_sip/research.md` | SIP convention and plate solving research. |
| `plan/20260805_01_stars/research-exif.md` | EXIF format and image loading research. |
| `python_prototype/` | Python validation prototype (this plan creates it). |

---

## 6. Commit Message Convention

All commits must follow **Conventional Commits** format with comprehensive descriptions:

```
<type>(<scope>): <short description>

<detailed body explaining what was changed and why>

<optional footer with breaking changes or issue refs>
```

### Types
- `feat` — New feature or capability
- `fix` — Bug fix
- `refactor` — Code restructuring without behavior change
- `test` — Adding or modifying tests
- `docs` — Documentation updates
- `chore` — Build, dependency, or tooling changes

### Scope
Use module names: `astrometry`, `star_finder`, `aberration`, `validation`, `exif`, `satellites`, `web`, `tui`, `cli`, `proto` (for Python prototype)

### Examples
```
feat(astrometry): add SIP polynomial distortion model

Implements forward and inverse SIP transformations for modeling
optical lens distortions. The SipDistortion struct stores A_p,q
and B_p,q polynomial coefficients up to 4th order, with methods
for applying distortion correction to pixel coordinates.

The implementation uses 64-bit floating point throughout to avoid
precision loss from cancellation at high polynomial orders, as
recommended by the SIP convention specification.
```

```
fix(astrometry): remove hardcoded 45° altitude assumption

Replace fixed center_alt=45.0° with iterative refinement using
least-squares minimization of star match residuals across the
(alt, az) parameter space. Initial estimate uses EXIF heading
and a geometric estimate from matched star positions.

This fixes false solutions when the camera points at altitudes
significantly different from 45°.
```

---

## 7. Dependencies to Add

### Rust Dependencies

| Crate | Version | GitHub | Purpose |
|-------|---------|--------|---------|
| `csv` | `1.3` | [BurntSushi/rust-csv](https://github.com/BurntSushi/rust-csv) | Loading external star catalog CSV |

### Python Dependencies (prototype)

| Package | Version | GitHub / PyPI | Purpose |
|---------|---------|---------------|---------|
| `twirl` | latest | [lgrcia/twirl](https://github.com/lgrcia/twirl) | Plate solving via Gaia catalog |
| `astropy` | latest | [astropy/astropy](https://github.com/astropy/astropy) | WCS, SIP, coordinate transforms |
| `astroquery` | latest | [astropy/astroquery](https://github.com/astropy/astroquery) | Gaia catalog queries |
| `photutils` | latest | [astropy/photutils](https://github.com/astropy/photutils) | DAOStarFinder star detection |
| `sep` | latest | [kbarbary/sep](https://github.com/kbarbary/sep) | Fast source extraction |
| `numpy` | latest | [numpy/numpy](https://github.com/numpy/numpy) | Numerical arrays |
| `matplotlib` | latest | [matplotlib/matplotlib](https://github.com/matplotlib/matplotlib) | Diagnostic plots |
| `Pillow` | latest | [python-pillow/Pillow](https://github.com/python-pillow/Pillow) | Image loading |
| `exifread` | latest | [ianare/exif-py](https://github.com/ianare/exif-py) | EXIF metadata extraction |
| `scipy` | latest | [scipy/scipy](https://github.com/scipy/scipy) | Polynomial fitting, optimization |

---

## 8. Docker Container Programs

The following should be pre-installed in the Docker container:

```dockerfile
RUN apt-get update && apt-get install -y \
    build-essential \
    clang \
    pkg-config \
    libssl-dev \
    python3 \
    python3-pip \
    python3-dev \
    && pip3 install uv
```
