# Implementation Plan: iPhone Star & Aberration Analyzer (`stars`)

## Project Context & Architecture

`stars` is a high-performance Rust application designed to read smartphone/iPhone astrophotography images (or sequences), detect stars while filtering landscape/foreground objects and image noise, determine star celestial coordinates via plate solving, detect optical camera aberration and atmospheric refraction, validate/correct embedded EXIF timestamp and heading data, and detect satellite streaks.

The tool provides both an interactive Terminal User Interface (TUI) binary using `ratatui` and a Web Application server using `axum` with an interactive browser dashboard.

---

## File Blueprint & Descriptions for AI Agents

When initializing context, an agent should examine the following files:

| File Path | Description |
|-----------|-------------|
| `deps.md` | List of external Rust crate dependencies and their GitHub organizations. |
| `Cargo.toml` | Manifest specifying dependencies, feature flags, and binary configurations. |
| `src/main.rs` | Application entry point handling CLI argument parsing (`clap`), launching TUI mode, CLI solver mode, or Web server mode. |
| `src/lib.rs` | Core library interface re-exporting all operational modules. |
| `src/exif/mod.rs` | EXIF metadata extraction module (`kamadak-exif`) for timestamp, GPS, heading, lens specs. |
| `src/image_loader/mod.rs` | Image loading, format conversion (JPEG, PNG, synthetic generator), and noise pre-filtering. |
| `src/star_finder/mod.rs` | Star detection engine using thresholding, connected components, sub-pixel 2D Gaussian/barycenter centroiding, and horizon/foreground masking. |
| `src/astrometry/mod.rs` | Plate solving engine, catalog matching against embedded Hipparcos/Yale bright star catalog (mag <= 6.5), KD-tree spatial lookup (`kiddo`), and horizontal/equatorial coordinate transformations. |
| `src/validation/mod.rs` | Verification engine comparing EXIF timestamp/heading against astrometric solution to detect and correct time/heading drifts. |
| `src/aberration/mod.rs` | Optical aberration modeling (radial distortion $k_1, k_2$, focal length estimation, PSF asymmetry/coma analysis across image field) and atmospheric refraction computation. |
| `src/satellites/mod.rs` | Streak detection engine (Hough transform line finder) and satellite trajectory matching with SGP4 orbit propagation. |
| `src/tui/mod.rs` | Interactive Terminal UI using `ratatui` displaying live dashboards, star maps, EXIF reports, aberration charts, and satellite tracks. |
| `src/web/mod.rs` | Axum HTTP REST API and static web application serving an interactive HTML5/SVG celestial visualizer. |
| `tests/integration_tests.rs` | End-to-end integration tests using synthetic starfield images with known EXIF, aberration, and satellite streaks. |

---

## Requirements & Feature Expansion Suggestions

### Core Requirements
1. **iPhone Image Ingestion**: Support JPEG, PNG, synthetic astronomical frames, EXIF decoding.
2. **Star Detection & Foreground Filtering**: Isolate stars from landscape ground, clouds, trees, and sensor noise using adaptive thresholding and gradient horizon detection.
3. **Sub-pixel Centroiding**: Calculate exact star center coordinates `(x, y)` using barycenter / 2D Gaussian fitting.
4. **Plate Solving & Celestial Coordinates**: Convert pixel coordinates to Altitude/Azimuth and Right Ascension / Declination (RA/Dec) using geometric matching against a bright star catalog (magnitude $\le 6.5$).
5. **EXIF Validation & Correction**: Validate embedded timestamp ($T_{\text{EXIF}}$) and camera compass heading ($\theta_{\text{EXIF}}$) against celestial orientation; return delta correction if drifted.
6. **Aberration & Atmosphere Modeling**: Model radial lens distortion ($k_1, k_2$), PSF coma/astigmatism across frame radius, and atmospheric refraction correction based on altitude angle.
7. **Satellite Track Detection**: Detect linear streaks across single or multi-frame sequences and propagate orbital elements via SGP4.
8. **Dual UI**: Provide a feature-packed Ratatui TUI application and an Axum Web UI with interactive canvas visualization.

### Expanded Feature Suggestions
- **Synthetic Test Bench**: Built-in test image generator producing realistic night sky photos with configurable stars, EXIF tags, lens distortion, atmospheric refraction, and satellite streaks for automated validation.
- **Export & Report Generator**: Export solved astronomical metadata as JSON, FITS-compatible WCS header strings, or Markdown summaries.
- **Sequence Stacking Preview**: Multi-frame median/mean alignment preview to boost signal-to-noise ratio in hand-held iPhone night mode sequences.

---

## Conventional Commit Rules

All git commits must strictly follow the Conventional Commits specification:

### Commit Format
```text
<type>(<scope>): <short summary>

<detailed description explaining why the change was made and what was changed>
```

### Commit Types
- `feat`: A new feature (e.g. `feat(star_finder): add sub-pixel 2D barycenter centroiding`)
- `fix`: A bug fix (e.g. `fix(exif): handle missing GPS altitude tag gracefully`)
- `test`: Adding or modifying tests (e.g. `test(astrometry): add synthetic plate solver integration test`)
- `docs`: Documentation updates (e.g. `docs(plan): add architecture diagram and implementation plan`)
- `refactor`: Code change that neither fixes a bug nor adds a feature

---

## Testing & Verification Strategy

1. **Unit Testing**:
   - `exif`: Test EXIF parsing with sample byte streams and fallback structures.
   - `star_finder`: Test thresholding, local maxima identification, barycenter precision on synthetic point sources.
   - `astrometry`: Test coordinate conversions (RA/Dec to Alt/Az, Sidereal Time calculation) and KD-tree catalog queries.
   - `aberration`: Test radial distortion polynomial calculation and atmospheric refraction formulas.
   - `satellites`: Test Hough transform streak detection on synthetic line segments.

2. **Integration Testing**:
   - `tests/integration_tests.rs`: Generate a synthetic astronomical frame with 20 known stars, radial distortion, timestamp, and a satellite streak. Run full pipeline and verify detected stars, EXIF validation delta, and satellite detection.

3. **Cargo Validation**:
   - `cargo test`: Ensure 100% test pass rate.
   - `cargo clippy -- -W clippy::all`: Clean lint checks without warnings.
   - `cargo fmt -- --check`: Code style compliance.
