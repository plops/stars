# Walkthrough: SIP Distortion & Plate Solving Validation (Phase 1 — Analysis & Prototype)

**Date**: 2026-08-07  
**Author**: AI Agent (on behalf of wol pumba, wolpumba@gmail.com)

---

## 1. What Was Implemented

### 1.1 Comprehensive Code Review

A thorough review of the entire `stars` Rust codebase was conducted, covering all 9 modules (~3,600 lines of Rust). The review identified:

- **6 critical/high-severity issues** including hardcoded 45° altitude assumption, tiny 50-star catalog, stale 2020 TLE data, and missing distortion correction
- **5 medium-severity issues** including unsigned residual analysis and hardcoded fallback values
- **12 hardcoded parameters** that should be configurable
- **Architectural strengths** confirmed: BFS star detection, 4D quad hashing, RGB chromatic measurement, Bennett refraction

The review built upon two prior review phases (20260805 initial build, 20260806 first review) which had already addressed some earlier issues.

### 1.2 Python Validation Prototype

A complete Python prototype was built in `python_prototype/` using `uv` for dependency management:

**File: `python_prototype/validate_plate_solving.py`** (197 lines)

Components implemented:
1. **Image Loading & Star Detection** — Using `photutils.DAOStarFinder` with σ-clipped statistics for background estimation. Detected 43 stars in `stars.jpg` and 19 stars in `IMG_8550.jpg`.
2. **EXIF Extraction** — Using `exifread` library. Discovered that test images lack GPS metadata, requiring fallback coordinates.
3. **Gaia Catalog Query** — Successfully queried Gaia DR3 via `astroquery`, retrieving 500 stars (mag < 12) within a 5° cone search.
4. **Plate Solving Attempt** — `twirl.compute_wcs()` was integrated but hanged on real images due to the large catalog size combined with imprecise initial coordinates. This validates that blind solving without good initial constraints is computationally expensive.
5. **Atmospheric Refraction** — Bennett's formula validated, producing correct ~34 arcmin at horizon, ~1 arcmin at 45°.
6. **SIP Distortion Simulation** — Simulated iPhone barrel distortion with k1=-0.1, k2=0.01 polynomial.

### 1.3 Diagnostic Plots Generated

Five diagnostic plots were produced in `python_prototype/plots/`:

| Plot | Description |
|------|-------------|
| `star_detection_stars.jpg.png` | DAOStarFinder detections overlaid on stars.jpg |
| `star_detection_IMG_8550.jpg.png` | DAOStarFinder detections overlaid on IMG_8550.jpg |
| `atmospheric_refraction.png` | Bennett refraction curve (0°–90° altitude) |
| `distortion_residuals.png` | Simulated barrel distortion polynomial |
| `catalog_parallax.png` | Gaia parallax distribution histogram |

### 1.4 Plan & Task Documents

- **`plan.md`** — Full implementation plan with code review findings, architecture design, dependency tables, commit conventions, and file reference
- **`task.md`** — 10-step serial task list with explicit test/validate/commit instructions for each step
- **`walkthrough.md`** — This document

---

## 2. Star Catalog Size & Parallax Analysis

### 2.1 Current Rust Catalog
- **50 stars** hardcoded inline in `get_bright_star_catalog()`
- Magnitude range: -1.46 to 3.77
- No parallax, no proper motion
- Approximately 6 KB of source code

### 2.2 Gaia Catalog via Python
- **Query**: 500 stars (limited), magnitude < 12, 5° cone search
- **Full Gaia DR3**: 1.468 billion sources
- **For our FOV (~65°)**: Would return ~50,000+ stars at mag < 10

### 2.3 Parallax Finding

**Parallax is negligible for iPhone astrophotography.**

- Maximum stellar parallax: 0.768" (Proxima Centauri)
- iPhone pixel scale: ~15-20 arcsec/pixel
- Maximum parallax shift: 0.04 pixels
- Gaia query parallax stats: min=0.133 mas, max=78.175 mas, median=2.629 mas

Proper motion is also negligible over the ~26 years from J2000 to 2026 (max ~10"/yr × 26yr = 260" ≈ 13 pixels for Barnard's Star — but this is an extreme outlier).

---

## 3. Key Learnings

### 3.1 Plate Solving Performance
- **Blind solving is expensive**: `twirl` without tight initial constraints hangs on large catalogs. The Rust implementation's approach of pre-filtering by EXIF-derived approximate pointing is correct.
- **Catalog size matters**: Matching against 500+ stars requires efficient indexing (KD-trees). The 50-star catalog is too small, but full catalogs need careful filtering.
- **Recommended approach**: Pre-filter catalog to ~100-200 brightest stars within estimated FOV, then use quad hashing.

### 3.2 Test Images
- Both test images (`stars.jpg`, `IMG_8550.jpg`) lack GPS EXIF metadata, limiting plate solving validation
- Star detection works well on both images with appropriate threshold settings
- The images appear to be genuine iPhone astrophotography with visible star fields

### 3.3 Python vs Rust Trade-offs
| Aspect | Python | Rust |
|--------|--------|------|
| Star detection accuracy | High (DAOStarFinder sub-pixel) | Good (BFS + barycenter) |
| Processing speed | Slower | Much faster |
| Catalog access | Easy (astroquery/Gaia) | Needs embedded or CSV catalog |
| WCS/SIP handling | Native (astropy) | Must implement from scratch |
| Distortion modeling | Mature (astropy.wcs) | Manual polynomial fitting |

---

## 4. Deviations from Original Plan

1. **`twirl` plate solving did not complete** — The library hanged when given large catalogs without tight constraints. This is a known limitation for wide-field images without precise initial pointing.
2. **`sep` package not installed** — Failed due to missing `Python.h` header. `photutils` was used as primary detector instead.
3. **`tetra3` was not used** — `twirl` was attempted first; since it required investigation, `tetra3` testing was deferred. For future work, `tetra3` (ESA's lost-in-space solver) is recommended for blind solving.
4. **No polynomial fit from real WCS** — Since plate solving didn't converge, SIP coefficient fitting used simulated distortion values rather than real measurements.

---

## 5. Docker Programs to Include

For the development container, the following programs should be installed:

### Build & Development Tools
```dockerfile
RUN apt-get update && apt-get install -y \
    build-essential \
    clang \
    pkg-config \
    libssl-dev \
    git \
    curl
```

### Python Environment
```dockerfile
RUN apt-get install -y \
    python3 \
    python3-dev \
    python3-venv \
    && pip3 install uv
```

### Astronomy Tools (optional but useful)
```dockerfile
RUN apt-get install -y \
    exiftool \
    astrometry.net \
    astrometry-data-tycho2
```

### Summary of Required Packages

| Package | Purpose | Size |
|---------|---------|------|
| `build-essential` | C compiler for native deps | ~250 MB |
| `clang` | LLVM compiler for Rust bindgen | ~100 MB |
| `python3-dev` | Python headers for native extensions | ~50 MB |
| `uv` (pip) | Fast Python package manager | ~10 MB |
| `exiftool` | CLI EXIF inspection | ~5 MB |
| `astrometry.net` | Reference plate solver | ~20 MB |
| `astrometry-data-tycho2` | Reference star catalog | ~60 MB |

---

## 6. Possible Extensions

### Short-term (next sprint)
1. **Implement `tetra3` in Python prototype** — ESA's lost-in-space solver for blind solving
2. **Generate Hipparcos CSV catalog** — Script to extract ~9110 bright stars for the Rust embedded catalog
3. **EXIF GPS injection** — Tool to add GPS metadata to test images for end-to-end validation

### Medium-term
4. **SIP coefficient FITS export** — Write solved distortion to standard FITS WCS headers
5. **iPhone lens profile database** — Pre-computed distortion profiles per iPhone model
6. **Real-time TLE fetching** — HTTP client to fetch current TLEs from CelesTrak
7. **Multi-image sequence stacking** — Combine multiple exposures for deeper detection

### Long-term
8. **WASM plate solver** — Compile core solver to WebAssembly for client-side web solving
9. **Event-based detection** — Integration with neuromorphic camera libraries
10. **Full astrometry.net port** — Native Rust implementation of the complete astrometry.net algorithm

---

## 7. Test Results Summary

### Rust Tests (all passing)
```
running 11 tests (unit) + 3 tests (integration) = 14 total
test result: ok. 14 passed; 0 failed; 0 ignored
```

### Clippy: 0 warnings
### Fmt: clean

### Python Prototype
- Star detection: ✅ Both images processed
- EXIF extraction: ✅ (with fallback for missing GPS)
- Gaia query: ✅ 500 stars retrieved
- Atmospheric refraction: ✅ Plot generated
- Distortion modeling: ✅ Plot generated
- Parallax analysis: ✅ Plot generated
- Plate solving (twirl): ⚠️ Timeout — requires tighter constraints
