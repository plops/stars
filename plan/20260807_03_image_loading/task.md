# Task: HEIC and Large JPEG Image Loader Support

## Goal
Fix image loading for `/workspace/src/IMG_8556.jpg` and `/workspace/src/IMG_8556.HEIC` across the Rust application (CLI & Web server) and validate in Python code.

## Requirements
- [x] Decode `.HEIC` files in Rust using `libheif-rs` and Python fallback.
- [x] Increase Axum web server body upload payload limit to 50MB to support large JPEGs (`IMG_8556.jpg`).
- [x] Update Python prototype with `pillow-heif` to validate JPEG and HEIC loading and EXIF parsing.
- [x] Verify full pipeline plate solving on both files (`IMG_8556.jpg` and `IMG_8556.HEIC`).
