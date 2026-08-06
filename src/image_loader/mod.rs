use crate::exif::{parse_exif_bytes, ExifMetadata};
use anyhow::{Context, Result};
use image::{GenericImageView, GrayImage, Luma, Rgb, RgbImage};
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct LoadedImage {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub gray: GrayImage,
    pub rgb: RgbImage,
    pub exif: ExifMetadata,
}

#[derive(Debug, Clone)]
pub struct SyntheticStarSpec {
    pub x: f64,
    pub y: f64,
    pub brightness: f64, // 0.0 to 255.0 peak
    pub fwhm: f64,       // Full width half maximum in pixels
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SyntheticSatelliteSpec {
    pub start_x: f64,
    pub start_y: f64,
    pub end_x: f64,
    pub end_y: f64,
    pub brightness: f64,
}

#[derive(Debug, Clone)]
pub struct SyntheticOptions {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub background_noise: f64,
    pub horizon_y: Option<u32>, // Height from top where ground landscape starts
    pub stars: Vec<SyntheticStarSpec>,
    pub satellite: Option<SyntheticSatelliteSpec>,
    pub radial_k1: f64,
    pub exif: ExifMetadata,
}

impl Default for SyntheticOptions {
    fn default() -> Self {
        let width = 1200;
        let height = 900;

        // Generate synthetic bright stars matching real summer constellations facing South in Munich (Altair, Tarazed, Alshain, Enif, Rotanev, Sualocin, Sadalmelik)
        let stars = vec![
            SyntheticStarSpec {
                x: 688.4,
                y: 362.3,
                brightness: 245.0,
                fwhm: 2.5,
                name: Some("Altair".into()),
            },
            SyntheticStarSpec {
                x: 682.1,
                y: 332.0,
                brightness: 230.0,
                fwhm: 2.3,
                name: Some("Tarazed".into()),
            },
            SyntheticStarSpec {
                x: 674.3,
                y: 399.5,
                brightness: 220.0,
                fwhm: 2.1,
                name: Some("Alshain".into()),
            },
            SyntheticStarSpec {
                x: 245.4,
                y: 335.5,
                brightness: 235.0,
                fwhm: 2.4,
                name: Some("Enif".into()),
            },
            SyntheticStarSpec {
                x: 545.2,
                y: 240.3,
                brightness: 215.0,
                fwhm: 2.0,
                name: Some("Rotanev".into()),
            },
            SyntheticStarSpec {
                x: 530.1,
                y: 218.4,
                brightness: 210.0,
                fwhm: 2.0,
                name: Some("Sualocin".into()),
            },
            SyntheticStarSpec {
                x: 330.1,
                y: 480.2,
                brightness: 225.0,
                fwhm: 2.2,
                name: Some("Sadalmelik".into()),
            },
            SyntheticStarSpec {
                x: 420.5,
                y: 600.3,
                brightness: 218.0,
                fwhm: 2.1,
                name: Some("Sadalsuud".into()),
            },
        ];

        Self {
            name: "synthetic_iphone_summer_sky.jpg".to_string(),
            width,
            height,
            background_noise: 12.0,
            horizon_y: Some(720), // Horizon line at 80% height (landscape at bottom)
            stars,
            satellite: Some(SyntheticSatelliteSpec {
                start_x: 100.0,
                start_y: 600.0,
                end_x: 1100.0,
                end_y: 100.0,
                brightness: 180.0,
            }),
            radial_k1: 0.0000001,
            exif: ExifMetadata::dummy_iphone_metadata(),
        }
    }
}

pub fn generate_synthetic_image(opts: &SyntheticOptions) -> LoadedImage {
    let mut rgb = RgbImage::new(opts.width, opts.height);
    let mut rng = StdRng::seed_from_u64(42);

    let cx = opts.width as f64 / 2.0;
    let cy = opts.height as f64 / 2.0;

    for y in 0..opts.height {
        for x in 0..opts.width {
            let mut val = opts.background_noise + rng.gen_range(-4.0..4.0);

            // Ground Landscape Mask (lower portion of the image with mountain silhouette)
            if let Some(horizon) = opts.horizon_y {
                if y >= horizon {
                    let hills = (x as f64 * 0.02).sin() * 20.0 + (x as f64 * 0.005).cos() * 40.0;
                    if (y as f64) > (horizon as f64 + hills) {
                        // Ground texture with dark foliage/landscape noise
                        val = 5.0 + rng.gen_range(0.0..10.0);
                    }
                }
            }

            val = val.clamp(0.0, 255.0);
            let u8_val = val as u8;
            rgb.put_pixel(
                x,
                y,
                Rgb([u8_val, u8_val, (u8_val as f64 * 1.1).min(255.0) as u8]),
            );
        }
    }

    // Render Stars with 2D Gaussian PSF and Radial Distortion
    for star in &opts.stars {
        let dx = star.x - cx;
        let dy = star.y - cy;
        let r2 = dx * dx + dy * dy;
        let distortion = 1.0 + opts.radial_k1 * r2;

        let distorted_x = cx + dx * distortion;
        let distorted_y = cy + dy * distortion;

        let sigma = star.fwhm / 2.355;
        let radius = (sigma * 4.0).ceil() as i32;

        let min_x = (distorted_x - radius as f64).max(0.0) as u32;
        let max_x = (distorted_x + radius as f64).min((opts.width - 1) as f64) as u32;
        let min_y = (distorted_y - radius as f64).max(0.0) as u32;
        let max_y = (distorted_y + radius as f64).min((opts.height - 1) as f64) as u32;

        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let pdx = px as f64 - distorted_x;
                let pdy = py as f64 - distorted_y;
                let dist_sq = pdx * pdx + pdy * pdy;

                let intensity = star.brightness * (-dist_sq / (2.0 * sigma * sigma)).exp();

                if intensity > 1.0 {
                    let pixel = rgb.get_pixel(px, py);
                    let new_r = (pixel[0] as f64 + intensity).min(255.0) as u8;
                    let new_g = (pixel[1] as f64 + intensity * 0.95).min(255.0) as u8;
                    let new_b = (pixel[2] as f64 + intensity * 1.05).min(255.0) as u8;
                    rgb.put_pixel(px, py, Rgb([new_r, new_g, new_b]));
                }
            }
        }
    }

    // Render Satellite Streak (linear path)
    if let Some(sat) = &opts.satellite {
        let steps = ((sat.end_x - sat.start_x).hypot(sat.end_y - sat.start_y) * 2.0) as usize;
        if steps > 0 {
            for i in 0..=steps {
                let t = i as f64 / steps as f64;
                let sx = sat.start_x + t * (sat.end_x - sat.start_x);
                let sy = sat.start_y + t * (sat.end_y - sat.start_y);

                let center_x = sx.round() as i32;
                let center_y = sy.round() as i32;

                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let px = center_x + dx;
                        let py = center_y + dy;
                        if px >= 0 && px < opts.width as i32 && py >= 0 && py < opts.height as i32 {
                            let pixel = rgb.get_pixel(px as u32, py as u32);
                            let new_r = (pixel[0] as f64 + sat.brightness).min(255.0) as u8;
                            let new_g = (pixel[1] as f64 + sat.brightness).min(255.0) as u8;
                            let new_b = (pixel[2] as f64 + sat.brightness).min(255.0) as u8;
                            rgb.put_pixel(px as u32, py as u32, Rgb([new_r, new_g, new_b]));
                        }
                    }
                }
            }
        }
    }

    // Convert RGB to Grayscale
    let mut gray = GrayImage::new(opts.width, opts.height);
    for y in 0..opts.height {
        for x in 0..opts.width {
            let p = rgb.get_pixel(x, y);
            let luma = (0.299 * p[0] as f64 + 0.587 * p[1] as f64 + 0.114 * p[2] as f64) as u8;
            gray.put_pixel(x, y, Luma([luma]));
        }
    }

    let mut exif = opts.exif.clone();
    exif.image_width = Some(opts.width);
    exif.image_height = Some(opts.height);

    LoadedImage {
        name: opts.name.clone(),
        width: opts.width,
        height: opts.height,
        gray,
        rgb,
        exif,
    }
}

pub fn load_image_from_bytes(name: &str, bytes: &[u8]) -> Result<LoadedImage> {
    let dyn_img = image::load_from_memory(bytes)
        .with_context(|| format!("Failed to decode image from memory for {}", name))?;

    let (width, height) = dyn_img.dimensions();
    let rgb = dyn_img.to_rgb8();
    let gray = dyn_img.to_luma8();

    let mut exif = parse_exif_bytes(bytes).unwrap_or_default();
    exif.image_width = Some(width);
    exif.image_height = Some(height);

    Ok(LoadedImage {
        name: name.to_string(),
        width,
        height,
        gray,
        rgb,
        exif,
    })
}

pub fn load_image_from_path(path: &Path) -> Result<LoadedImage> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("Failed to read file at path {}", path.display()))?;
    let filename = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "image".to_string());
    load_image_from_bytes(&filename, &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_synthetic_image() {
        let opts = SyntheticOptions::default();
        let img = generate_synthetic_image(&opts);

        assert_eq!(img.width, 1200);
        assert_eq!(img.height, 900);
        assert_eq!(img.gray.dimensions(), (1200, 900));
        assert_eq!(img.exif.make.as_deref(), Some("Apple"));
    }
}
