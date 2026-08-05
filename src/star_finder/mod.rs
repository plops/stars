use image::GrayImage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedStar {
    pub id: usize,
    pub x: f64,
    pub y: f64,
    pub intensity: f64,
    pub peak_brightness: u8,
    pub fwhm: f64,
    pub snr: f64,
    pub elongation: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionSettings {
    pub sigma_threshold: f64,
    pub min_area: usize,
    pub max_area: usize,
    pub max_elongation: f64,
    pub mask_horizon: bool,
}

impl Default for DetectionSettings {
    fn default() -> Self {
        Self {
            sigma_threshold: 2.5,
            min_area: 2,
            max_area: 500,
            max_elongation: 2.8,
            mask_horizon: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionResult {
    pub stars: Vec<DetectedStar>,
    pub background_mean: f64,
    pub background_std: f64,
    pub horizon_y: Option<u32>,
    pub rejected_blobs: usize,
}

pub fn detect_stars(img: &GrayImage, settings: &DetectionSettings) -> DetectionResult {
    let (width, height) = img.dimensions();

    // 1. Detect Landscape Horizon Y (if enabled)
    let horizon_y = if settings.mask_horizon {
        detect_horizon_y(img)
    } else {
        None
    };

    let effective_height = horizon_y.unwrap_or(height);

    // 2. Global Sky Background Stats
    let (global_bg_mean, global_bg_std) = estimate_background(img, effective_height);

    // Absolute Noise Floor for Genuine Astronomical Stars
    let global_noise_floor = global_bg_mean + 1.8 * global_bg_std;

    // 3. Construct Adaptive Local Background Grid (16x12 Mesh)
    let grid_cols = 16u32;
    let grid_rows = 12u32;
    let cell_w = (width / grid_cols).max(1);
    let cell_h = (effective_height / grid_rows).max(1);

    let mut local_means = vec![global_bg_mean; (grid_cols * grid_rows) as usize];
    let mut local_stds = vec![global_bg_std; (grid_cols * grid_rows) as usize];

    for gy in 0..grid_rows {
        for gx in 0..grid_cols {
            let start_x = gx * cell_w;
            let end_x = ((gx + 1) * cell_w).min(width);
            let start_y = gy * cell_h;
            let end_y = ((gy + 1) * cell_h).min(effective_height);

            let mut cell_pixels = Vec::new();
            for y in (start_y..end_y).step_by(2) {
                for x in (start_x..end_x).step_by(2) {
                    cell_pixels.push(img.get_pixel(x, y)[0] as f64);
                }
            }

            if !cell_pixels.is_empty() {
                cell_pixels.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let median = cell_pixels[cell_pixels.len() / 2];

                let mut mads: Vec<f64> = cell_pixels.iter().map(|p| (p - median).abs()).collect();
                mads.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let mad = mads[mads.len() / 2];
                let std_dev = (mad * 1.4826).max(1.5);

                let g_idx = (gy * grid_cols + gx) as usize;
                local_means[g_idx] = median;
                local_stds[g_idx] = std_dev;
            }
        }
    }

    // 4. Scan Image for Genuine Astronomical Stars
    let mut stars = Vec::new();
    let mut rejected_blobs = 0;

    for y in 2..(effective_height - 2) {
        let gy = (y / cell_h).min(grid_rows - 1);
        for x in 2..(width - 2) {
            let gx = (x / cell_w).min(grid_cols - 1);
            let g_idx = (gy * grid_cols + gx) as usize;
            let bg_m = local_means[g_idx];
            let bg_s = local_stds[g_idx];

            let local_threshold = bg_m + settings.sigma_threshold * bg_s;
            let val = img.get_pixel(x, y)[0] as f64;

            // Global Noise Floor Filter: Must exceed global noise floor (eliminates tree foliage noise)
            if val < global_noise_floor {
                continue;
            }

            // Strict 3x3 Local Maximum Peak
            if val >= local_threshold
                && val > img.get_pixel(x - 1, y)[0] as f64
                && val > img.get_pixel(x + 1, y)[0] as f64
                && val > img.get_pixel(x, y - 1)[0] as f64
                && val > img.get_pixel(x, y + 1)[0] as f64
                && val >= img.get_pixel(x - 1, y - 1)[0] as f64
                && val >= img.get_pixel(x + 1, y - 1)[0] as f64
                && val >= img.get_pixel(x - 1, y + 1)[0] as f64
                && val >= img.get_pixel(x + 1, y + 1)[0] as f64
            {
                // Calculate 3x3 Surrounding Average for Point-Source PSF Sharpness
                let mut surround_sum = 0.0;
                let mut surround_count = 0.0;

                for ny in (y - 1)..=(y + 1) {
                    for nx in (x - 1)..=(x + 1) {
                        if nx != x || ny != y {
                            surround_sum += img.get_pixel(nx, ny)[0] as f64;
                            surround_count += 1.0;
                        }
                    }
                }

                let surround_avg = surround_sum / surround_count;
                let peak_signal = val - bg_m;
                let surround_signal = (surround_avg - bg_m).max(0.1);

                // PSF Sharpness Test: Peak must be brighter than surrounding border
                let sharpness_ratio = peak_signal / surround_signal;
                if sharpness_ratio < 1.02 {
                    rejected_blobs += 1;
                    continue;
                }

                // Sub-pixel 3x3 Centroid (Barycenter) calculation
                let mut sum_i = 0.0;
                let mut sum_x = 0.0;
                let mut sum_y = 0.0;

                for ny in (y - 1)..=(y + 1) {
                    for nx in (x - 1)..=(x + 1) {
                        let pval = img.get_pixel(nx, ny)[0] as f64;
                        let weight = (pval - bg_m).max(0.1);
                        sum_i += weight;
                        sum_x += nx as f64 * weight;
                        sum_y += ny as f64 * weight;
                    }
                }

                if sum_i > 0.0 {
                    let sub_x = sum_x / sum_i;
                    let sub_y = sum_y / sum_i;

                    // Calculate Second Moments for Elongation & FWHM
                    let mut mu_xx = 0.0;
                    let mut mu_yy = 0.0;
                    let mut mu_xy = 0.0;

                    for ny in (y - 1)..=(y + 1) {
                        for nx in (x - 1)..=(x + 1) {
                            let pval = img.get_pixel(nx, ny)[0] as f64;
                            let weight = (pval - bg_m).max(0.1);
                            let dx = nx as f64 - sub_x;
                            let dy = ny as f64 - sub_y;
                            mu_xx += dx * dx * weight;
                            mu_yy += dy * dy * weight;
                            mu_xy += dx * dy * weight;
                        }
                    }

                    mu_xx /= sum_i;
                    mu_yy /= sum_i;
                    mu_xy /= sum_i;

                    let delta = ((mu_xx - mu_yy).powi(2) + 4.0 * mu_xy * mu_xy).sqrt();
                    let lambda1 = (mu_xx + mu_yy + delta) / 2.0;
                    let lambda2 = ((mu_xx + mu_yy - delta) / 2.0).max(1e-4);
                    let elongation = (lambda1 / lambda2).sqrt();

                    if elongation <= settings.max_elongation {
                        let fwhm = 2.355 * (lambda1 + lambda2).sqrt() / std::f64::consts::SQRT_2;
                        let snr = peak_signal / bg_s.max(1.0);

                        stars.push(DetectedStar {
                            id: stars.len() + 1,
                            x: sub_x,
                            y: sub_y,
                            intensity: sum_i,
                            peak_brightness: val as u8,
                            fwhm: fwhm.clamp(1.2, 8.0),
                            snr,
                            elongation,
                        });
                    } else {
                        rejected_blobs += 1;
                    }
                }
            }
        }
    }

    DetectionResult {
        stars,
        background_mean: global_bg_mean,
        background_std: global_bg_std,
        horizon_y,
        rejected_blobs,
    }
}

// Detect landscape horizon by analyzing vertical intensity gradient variance & edge texture from bottom up
fn detect_horizon_y(img: &GrayImage) -> Option<u32> {
    let (width, height) = img.dimensions();
    if height < 100 {
        return None;
    }

    let start_y = (height as f64 * 0.4) as u32;

    for y in (start_y..height - 10).rev() {
        let mut row_sum = 0.0;
        let mut row_sq_sum = 0.0;
        let mut edge_sum = 0.0;

        for x in (5..width - 5).step_by(4) {
            let val = img.get_pixel(x, y)[0] as f64;
            let val_above = img.get_pixel(x, y - 2)[0] as f64;
            let diff = (val - val_above).abs();

            row_sum += val;
            row_sq_sum += val * val;
            edge_sum += diff;
        }

        let count = ((width - 10) / 4) as f64;
        let mean = row_sum / count;
        let variance = (row_sq_sum / count) - (mean * mean);
        let avg_edge = edge_sum / count;

        if (mean > 32.0 && variance > 110.0) || avg_edge > 12.0 {
            return Some(y);
        }
    }

    None
}

fn estimate_background(img: &GrayImage, max_y: u32) -> (f64, f64) {
    let (width, _) = img.dimensions();
    let mut pixels = Vec::with_capacity((width * max_y / 10) as usize);

    for y in (0..max_y).step_by(3) {
        for x in (0..width).step_by(3) {
            pixels.push(img.get_pixel(x, y)[0] as f64);
        }
    }

    if pixels.is_empty() {
        return (10.0, 2.0);
    }

    pixels.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = pixels[pixels.len() / 2];

    let mut mads: Vec<f64> = pixels.iter().map(|p| (p - median).abs()).collect();
    mads.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mad = mads[mads.len() / 2];

    let std_dev = mad * 1.4826;
    (median, std_dev.max(1.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image_loader::{generate_synthetic_image, SyntheticOptions};

    #[test]
    fn test_detect_stars_synthetic() {
        let opts = SyntheticOptions::default();
        let loaded = generate_synthetic_image(&opts);

        let settings = DetectionSettings::default();
        let result = detect_stars(&loaded.gray, &settings);

        assert!(
            !result.stars.is_empty(),
            "Should detect stars in synthetic image"
        );
        assert!(
            result.stars.len() >= 5,
            "Expected at least 5 stars detected, got {}",
            result.stars.len()
        );

        if let Some(hy) = result.horizon_y {
            for star in &result.stars {
                assert!(
                    star.y < hy as f64,
                    "All detected stars must be above horizon y={hy}"
                );
            }
        }
    }
}
