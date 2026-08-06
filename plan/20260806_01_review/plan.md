# Software Architecture & Code Review Plan for Astrophotography Software (`stars`)

**Author**: wol pumba (`wolpumba@gmail.com`) / AI Pair Programmer  
**Date**: 2026-08-06  
**Target Repository**: `/workspace/src/stars`  

---

## 1. Executive Summary & Codebase Review Findings

Following an in-depth code review of the `stars` codebase, several critical architectural flaws, synthetic shortcuts, and maintainability issues were identified:

1. **Plate Solver Integrity Flaw**:
   - In [`src/astrometry/mod.rs`](file:///workspace/src/stars/src/astrometry/mod.rs), if catalog matching returns fewer than 3 matches, a synthetic fallback forcibly creates fake matches (`residual_pixels = 0.8`) and marks the solution as `solved = true`.
   - The solver lacks true **Geometric Quad Hashing** (as researched in `research.md`). It currently depends on rough initial location/heading estimates.
   - The catalog contains only 19 stars, making plate solving unreliable for real astrophotographs with different FOVs or star fields.

2. **Star Finder Over-segmentation & Simple Barycenter**:
   - In [`src/star_finder/mod.rs`](file:///workspace/src/stars/src/star_finder/mod.rs), star centroids and FWHM are computed over a fixed $3 \times 3$ pixel box around local maxima. Bright or slightly out-of-focus stars span $5\text{--}15$ pixels, causing over-segmentation into multiple false peaks and skewed centroid calculations.

3. **Synthetic Optical Aberration Metrics**:
   - In [`src/aberration/mod.rs`](file:///workspace/src/stars/src/aberration/mod.rs), metrics like `astigmatism_factor` ($0.8 \times \text{coma}$) and `chromatic_aberration_px` ($1.5 \times \text{coma}$) use hardcoded synthetic multipliers rather than actual measurements. True chromatic aberration (RGB channel centroid displacement near image edges) is not measured.

4. **Hardcoded Satellite TLE & Fallback**:
   - In [`src/satellites/mod.rs`](file:///workspace/src/stars/src/satellites/mod.rs), SGP4 orbit propagation uses hardcoded TLE strings from 2020 and a hardcoded propagation offset (`MinutesSinceEpoch(15.0)`). If propagation fails, fake satellite coordinates `(6700.0, 1200.0, 3400.0)` are returned.

5. **EXIF Metadata & Orientation Handling**:
   - In [`src/exif/mod.rs`](file:///workspace/src/stars/src/exif/mod.rs), EXIF image orientation (tag `0x0112`) is ignored. Mobile photos taken in portrait or rotated modes skew coordinate systems.

---

## 2. Requirements & Feature Proposal

To make this software production-ready, maintainable, and high-performing, the following enhancements are proposed:

### Core Requirements & Enhancements
- **Geometric Quad Hashing (4D KD-Tree)**: Implement scale- and rotation-invariant quad hash extraction $((u_1, v_1, u_2, v_2))$ using top star centroids and query against a 4D `kiddo::KdTree`.
- **Expanded Constellation & Star Catalog**: Expand catalog to 45+ primary stars across all major constellations (Orion, Ursa Major, Cassiopeia, Cygnus, Scorpius, Leo, Pegasus, etc.).
- **Connected Component Star Detection**: Implement region grouping for star blobs to accurately measure sub-pixel centroids, FWHM, and SNR across full blob extents.
- **Empirical RGB Chromatic Aberration & Lens Distortion**: Compute channel-wise centroid shifts (Red vs Blue) across image radii $r/R_{\text{max}}$ for true chromatic aberration and polynomial radial distortion $k_1, k_2$.
- **Multi-Satellite TLE Registry & Dynamic Propagation**: Implement a TLE registry (ISS, Hubble, Starlink, Tiangong) and propagate orbits dynamically to image `timestamp_utc`.
- **Comprehensive EXIF & Orientation Support**: Support EXIF orientation transforms and calculate exact time drift ($15^\circ/\text{hr}$) from plate solving celestial RA deltas.
- **Web App & TUI Enhancements**: Provide interactive controls in Axum web UI (sigma slider, layer overlays for catalog stars, satellite tracks, vector fields) and rich Ratatui dashboard rendering.

---

## 3. Context Files for AI Agents

An independent AI agent working on this codebase should inspect the following key files:

| File Path | Description |
|-----------|-------------|
| [`src/lib.rs`](file:///workspace/src/stars/src/lib.rs) | Crate module declarations re-exporting all sub-modules. |
| [`src/astrometry/mod.rs`](file:///workspace/src/stars/src/astrometry/mod.rs) | Star catalog, Julian Date, equatorial to alt-az projection, Geometric Quad Hashing, and plate solver. |
| [`src/star_finder/mod.rs`](file:///workspace/src/stars/src/star_finder/mod.rs) | Adaptive local background estimation, connected component star detection, centroiding, FWHM & SNR. |
| [`src/aberration/mod.rs`](file:///workspace/src/stars/src/aberration/mod.rs) | Optical quality scoring, radial polynomial fitting ($k_1, k_2$), RGB chromatic aberration, and atmospheric refraction. |
| [`src/satellites/mod.rs`](file:///workspace/src/stars/src/satellites/mod.rs) | RANSAC streak detection, multi-satellite TLE database, and SGP4 orbit propagation. |
| [`src/exif/mod.rs`](file:///workspace/src/stars/src/exif/mod.rs) | EXIF parsing via `kamadak-exif`, orientation extraction, and timestamp/GPS metadata. |
| [`src/validation/mod.rs`](file:///workspace/src/stars/src/validation/mod.rs) | EXIF timestamp drift validation and compass heading error estimation. |
| [`src/image_loader/mod.rs`](file:///workspace/src/stars/src/image_loader/mod.rs) | Image decoding, synthetic image generator, and R/G/B channel extraction. |
| [`src/web/mod.rs`](file:///workspace/src/stars/src/web/mod.rs) | Axum REST API endpoints, image pipeline execution, base64 encoding, and web viewer interface. |
| [`src/tui/mod.rs`](file:///workspace/src/stars/src/tui/mod.rs) | Ratatui terminal dashboard, canvas rendering of celestial star maps and streak overlays. |
| [`deps.md`](file:///workspace/src/stars/deps.md) | Registry of crate dependencies, version constraints, and GitHub organizations. |

---

## 4. Conventional Commit Specification

All commits must follow the **Conventional Commits** standard:

```text
<type>(<scope>): <short description>

<comprehensive description explaining why the change was made, technical details, and verification steps>
```

### Commit Types
- `feat`: A new feature (e.g. geometric quad hashing, multi-satellite TLE propagation).
- `fix`: A bug fix (e.g. removing synthetic fallback matches, fixing orientation).
- `refactor`: Code restructuring without changing external behavior.
- `test`: Adding or updating unit/integration tests.
- `docs`: Documentation updates (`plan.md`, `task.md`, `walkthrough.md`, `deps.md`).

### Example Commit Message
```text
feat(astrometry): implement 4D geometric quad hashing plate solver

Replaced single 2D projection lookup with scale- and rotation-invariant
geometric quad hashing. Extracted 4D hashes (u1, v1, u2, v2) for top star
quads and built a 4D KdTree search index using `kiddo`. Removed hardcoded
synthetic fallback matches to ensure authentic astrometric verification.

Verified with `cargo test astrometry::tests::test_quad_hashing`.
```

---

## 5. Crate Usage Examples (Queried via DeepWiki MCP)

### `kiddo` (4D KdTree Quad Hashing & Spatial Search)
```rust
use kiddo::{KdTree, SquaredEuclidean};

// 4D KdTree for Quad Hashes [u1, v1, u2, v2]
let mut quad_tree: KdTree<f64, usize, 4> = KdTree::new();
quad_tree.add(&[0.25, 0.40, 0.60, 0.15], 0); // index of catalog quad

let query_quad = [0.26, 0.41, 0.59, 0.14];
let nearest = quad_tree.nearest_one::<SquaredEuclidean>(&query_quad);
if nearest.distance.sqrt() < 0.05 {
    println!("Matched catalog quad index: {}", nearest.item);
}
```

### `sgp4` (Satellite Orbit Propagation)
```rust
use sgp4::{Constants, Elements, MinutesSinceEpoch};

let line1 = "1 25544U 98067A   20350.51341435  .00001432  00000-0  34324-4 0  9990";
let line2 = "2 25544  51.6448 147.2862 0002641 120.4852 301.7645 15.49187318259468";

let elements = Elements::from_tle(None, line1.as_bytes(), iss_line2.as_bytes()).unwrap();
let constants = Constants::from_elements(&elements).unwrap();
let pred = constants.propagate(MinutesSinceEpoch(10.0)).unwrap();
println!("ECEF position km: {:?}", pred.position);
```

### `kamadak-exif` (Reading EXIF Metadata)
```rust
use exif::{In, Reader, Tag, Value};
use std::io::Cursor;

let mut cursor = Cursor::new(image_bytes);
let exif = Reader::new().read_from_container(&mut cursor).unwrap();
if let Some(field) = exif.get_field(Tag::Orientation, In::PRIMARY) {
    println!("Image Orientation: {}", field.display_value());
}
```

---
