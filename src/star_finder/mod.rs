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
            sigma_threshold: 4.0,
            min_area: 3,
            max_area: 400,
            max_elongation: 2.2,
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

    // 2. Estimate Sky Background Mean and StdDev (Sigma Clipping)
    let (bg_mean, bg_std) = estimate_background(img, effective_height);
    let threshold = bg_mean + settings.sigma_threshold * bg_std;

    // 3. Find Connected Components above Threshold
    let mut visited = vec![false; (width * effective_height) as usize];
    let mut stars = Vec::new();
    let mut rejected_blobs = 0;

    for y in 0..effective_height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            if visited[idx] {
                continue;
            }
            visited[idx] = true;

            let pixel_val = img.get_pixel(x, y)[0] as f64;
            if pixel_val > threshold {
                // BFS to collect component pixels
                let mut component = Vec::new();
                let mut queue = vec![(x, y)];

                while let Some((cx, cy)) = queue.pop() {
                    let cval = img.get_pixel(cx, cy)[0] as f64;
                    component.push((cx, cy, cval));

                    for (dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
                        let nx = cx as i32 + dx;
                        let ny = cy as i32 + dy;

                        if nx >= 0 && nx < width as i32 && ny >= 0 && ny < effective_height as i32 {
                            let nidx = (ny as u32 * width + nx as u32) as usize;
                            if !visited[nidx] {
                                let nval = img.get_pixel(nx as u32, ny as u32)[0] as f64;
                                if nval > threshold {
                                    visited[nidx] = true;
                                    queue.push((nx as u32, ny as u32));
                                }
                            }
                        }
                    }
                }

                // 4. Evaluate Component Blob Criteria
                if component.len() >= settings.min_area && component.len() <= settings.max_area {
                    let mut sum_i = 0.0;
                    let mut sum_x = 0.0;
                    let mut sum_y = 0.0;
                    let mut peak = 0u8;

                    for &(px, py, val) in &component {
                        let weight = val - bg_mean;
                        sum_i += weight;
                        sum_x += px as f64 * weight;
                        sum_y += py as f64 * weight;
                        if val as u8 > peak {
                            peak = val as u8;
                        }
                    }

                    if sum_i > 0.0 {
                        let star_x = sum_x / sum_i;
                        let star_y = sum_y / sum_i;

                        // Calculate Second Moments for Elongation & FWHM
                        let mut mu_xx = 0.0;
                        let mut mu_yy = 0.0;
                        let mut mu_xy = 0.0;

                        for &(px, py, val) in &component {
                            let weight = val - bg_mean;
                            let dx = px as f64 - star_x;
                            let dy = py as f64 - star_y;
                            mu_xx += dx * dx * weight;
                            mu_yy += dy * dy * weight;
                            mu_xy += dx * dy * weight;
                        }

                        mu_xx /= sum_i;
                        mu_yy /= sum_i;
                        mu_xy /= sum_i;

                        let delta = ((mu_xx - mu_yy).powi(2) + 4.0 * mu_xy * mu_xy).sqrt();
                        let lambda1 = (mu_xx + mu_yy + delta) / 2.0;
                        let lambda2 = ((mu_xx + mu_yy - delta) / 2.0).max(1e-4);

                        let elongation = (lambda1 / lambda2).sqrt();

                        // Reject satellite streaks or extended lines
                        if elongation <= settings.max_elongation {
                            let fwhm =
                                2.355 * (lambda1 + lambda2).sqrt() / std::f64::consts::SQRT_2;
                            let snr = (peak as f64 - bg_mean) / bg_std.max(1.0);

                            stars.push(DetectedStar {
                                id: stars.len() + 1,
                                x: star_x,
                                y: star_y,
                                intensity: sum_i,
                                peak_brightness: peak,
                                fwhm,
                                snr,
                                elongation,
                            });
                        } else {
                            rejected_blobs += 1;
                        }
                    }
                } else {
                    rejected_blobs += 1;
                }
            }
        }
    }

    DetectionResult {
        stars,
        background_mean: bg_mean,
        background_std: bg_std,
        horizon_y,
        rejected_blobs,
    }
}

// Detect landscape horizon by analyzing vertical intensity gradient variance from bottom up
fn detect_horizon_y(img: &GrayImage) -> Option<u32> {
    let (width, height) = img.dimensions();
    if height < 100 {
        return None;
    }

    // Scan from bottom 40% of image upwards to find transition from high variance/dense pixels to sky
    let start_y = (height as f64 * 0.5) as u32;

    for y in (start_y..height - 10).rev() {
        let mut row_sum = 0.0;
        let mut row_sq_sum = 0.0;

        for x in (0..width).step_by(5) {
            let val = img.get_pixel(x, y)[0] as f64;
            row_sum += val;
            row_sq_sum += val * val;
        }

        let count = (width / 5) as f64;
        let mean = row_sum / count;
        let variance = (row_sq_sum / count) - (mean * mean);

        // Ground landscape typically has higher brightness/variance or structured features
        if mean > 40.0 && variance > 200.0 {
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

    // 50th percentile (median) as background mean estimate
    let median = pixels[pixels.len() / 2];

    // Standard deviation estimated from Median Absolute Deviation (MAD)
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

        // Verify horizon detection masked lower portion
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
