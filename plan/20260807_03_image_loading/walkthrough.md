# Walkthrough: High-Resolution JPEG & HEIC Image Loader Support

**Date:** 2026-08-07  
**Client:** Wol Pumba (`wolpumba@gmail.com`)  
**Scope:** Fix loading issues for large JPEG images (`IMG_8556.jpg`) and HEIC images (`IMG_8556.HEIC`), implement HEIF/HEVC image decoding support in Rust and Python, and increase Axum web server payload limits.

---

## 1. Summary of Completed Tasks

We have resolved all image loading failures for high-resolution iPhone camera files `/workspace/src/IMG_8556.jpg` (2.79 MB JPEG) and `/workspace/src/IMG_8556.HEIC` (1.91 MB HEIC):

1. **Rust HEIC Image Decoder (`src/image_loader/mod.rs`)**:
   - Added `libheif-rs` (`2.7.0`) with `embedded-libheif` and `image` features to `Cargo.toml`.
   - Implemented `load_heic_from_bytes()` in `src/image_loader/mod.rs` to parse HEIF container structures and decode uncompressed RGB frame buffers into `LoadedImage` structs.
   - Added a robust Python fallback (`decode_heic_via_python`) using `pillow-heif` for HEVC compressed frames when native decoder plugins are unavailable.

2. **Axum Web Server Payload Limit Fix (`src/web/mod.rs`)**:
   - Identified that Axum's default `DefaultBodyLimit` is 2 MB (2,097,152 bytes), causing uploads of `IMG_8556.jpg` (2.79 MB) to fail with HTTP `413 Payload Too Large`.
   - Added `.layer(DefaultBodyLimit::max(50 * 1024 * 1024))` to the Axum Router in `src/web/mod.rs`, expanding the maximum upload payload limit to 50 MB for large JPEG and HEIC images.

3. **Python Toolchain Validation (`python_prototype/`)**:
   - Added `pillow-heif` (`1.5.0`) dependency via `uv add pillow-heif` to `python_prototype/pyproject.toml`.
   - Updated `python_prototype/validate_plate_solving.py` with `pillow_heif.register_heif_opener()` to support native HEIC opening via Pillow.
   - Validated both `/workspace/src/IMG_8556.jpg` (4032x3024 RGB) and `/workspace/src/IMG_8556.HEIC` (4032x3024 RGB, EXIF: iPhone 11) in Python.

4. **Verification & Toolchain Testing**:
   - Cleanly passed `cargo test`: 19 unit tests and 4 integration tests passed.
   - Verified CLI execution for both `/workspace/src/IMG_8556.jpg` (1618 detected stars, SUCCESS plate solve) and `/workspace/src/IMG_8556.HEIC` (1533 detected stars, SUCCESS plate solve).

---

## 2. Technical Implementation Details

### Rust HEIC Decoding & Fallback Strategy

```rust
// src/image_loader/mod.rs
pub fn load_heic_from_bytes(name: &str, bytes: &[u8]) -> Result<LoadedImage> {
    // 1. Try native libheif-rs decode
    let native_decode_result: Result<(RgbImage, u32, u32)> = (|| {
        let ctx = libheif_rs::HeifContext::read_from_bytes(bytes)?;
        let handle = ctx.primary_image_handle()?;
        let lib_heif = libheif_rs::LibHeif::new();
        let image = lib_heif.decode(&handle, libheif_rs::ColorSpace::Rgb(libheif_rs::RgbChroma::Rgb), None)?;
        // Extract interleaved RGB plane pixels
        ...
    })();

    let (rgb, width, height) = match native_decode_result {
        Ok(res) => res,
        Err(_) => {
            // 2. Fallback to Python pillow-heif process if native codec plugin missing
            let png_bytes = decode_heic_via_python(bytes)?;
            let dyn_img = image::load_from_memory(&png_bytes)?;
            let (w, h) = dyn_img.dimensions();
            (dyn_img.to_rgb8(), w, h)
        }
    };
    ...
}
```

---

## 3. Empirical Verification Results

```text
========================================================
✦ iPHONE STAR RECOGNITION & ABERRATION ANALYSIS ✦
========================================================
Image Name:           IMG_8556.HEIC
Resolution:           4032x3024 px
Camera Model:         iPhone 11
Detected Stars:       1533
Landscape Horizon Y:  Some(1876)
Plate Solve Status:   SUCCESS
Matched Catalog Stars: 113
RMS Residual Error:   11.54 px
EXIF Validation:      EXIF metadata verified cleanly. Timestamp drift: -4.0s, Compass heading error: -0.02°
Radial Distortion k1: 0.050000
Atmospheric Refraction: 0.99 arcmin
Optical Quality Score: 10.0 / 100
Detected Satellites:  0
========================================================
```

---

## 4. Summary of Changed Files

- [`Cargo.toml`](file:///workspace/src/stars/Cargo.toml): Added `libheif-rs` (`embedded-libheif`, `image` features).
- [`Cargo.lock`](file:///workspace/src/stars/Cargo.lock): Updated cargo lockfile.
- [`src/image_loader/mod.rs`](file:///workspace/src/stars/src/image_loader/mod.rs): Added HEIC decoding and Python fallback.
- [`src/web/mod.rs`](file:///workspace/src/stars/src/web/mod.rs): Increased Axum body limit layer to 50 MB (`DefaultBodyLimit::max`).
- [`python_prototype/pyproject.toml`](file:///workspace/src/stars/python_prototype/pyproject.toml): Added `pillow-heif`.
- [`python_prototype/uv.lock`](file:///workspace/src/stars/python_prototype/uv.lock): Updated lockfile.
- [`python_prototype/validate_plate_solving.py`](file:///workspace/src/stars/python_prototype/validate_plate_solving.py): Registered `pillow-heif` and added `IMG_8556` test files.
