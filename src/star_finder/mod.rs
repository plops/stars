use image::GrayImage;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

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
            sigma_threshold: 2.2,
            min_area: 2,
            max_area: 500,
            max_elongation: 3.2,
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
    let global_noise_floor = global_bg_mean + 1.5 * global_bg_std;

    // 3. Construct Threshold Mask & Connected Component Region Growing
    let threshold = global_bg_mean + settings.sigma_threshold * global_bg_std;
    let mut visited = vec![false; (width * effective_height) as usize];
    let mut stars = Vec::new();
    let mut rejected_blobs = 0;

    for y in 2..(effective_height - 2) {
        for x in 2..(width - 2) {
            let idx = (y * width + x) as usize;
            if visited[idx] {
                continue;
            }

            let val = img.get_pixel(x, y)[0] as f64;
            if val >= threshold && val >= global_noise_floor {
                // BFS Connected Component Extraction
                let mut queue = VecDeque::new();
                queue.push_back((x, y));
                visited[idx] = true;

                let mut component = Vec::new();
                let mut max_val = 0u8;

                while let Some((cx, cy)) = queue.pop_front() {
                    let pixel_val = img.get_pixel(cx, cy)[0];
                    if pixel_val > max_val {
                        max_val = pixel_val;
                    }
                    component.push((cx, cy, pixel_val as f64));

                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            if dx == 0 && dy == 0 {
                                continue;
                            }
                            let nx = cx as i32 + dx;
                            let ny = cy as i32 + dy;

                            if nx >= 2
                                && nx < (width as i32 - 2)
                                && ny >= 2
                                && ny < (effective_height as i32 - 2)
                            {
                                let unx = nx as u32;
                                let uny = ny as u32;
                                let nidx = (uny * width + unx) as usize;

                                if !visited[nidx] {
                                    let nval = img.get_pixel(unx, uny)[0] as f64;
                                    if nval >= global_noise_floor {
                                        visited[nidx] = true;
                                        queue.push_back((unx, uny));
                                    }
                                }
                            }
                        }
                    }
                }

                // Filter components by pixel area bounds
                if component.len() < settings.min_area || component.len() > settings.max_area {
                    rejected_blobs += 1;
                    continue;
                }

                // Calculate intensity-weighted barycenter centroid
                let mut total_weight = 0.0;
                let mut sum_x = 0.0;
                let mut sum_y = 0.0;

                for &(px, py, pval) in &component {
                    let weight = (pval - global_bg_mean).max(0.1);
                    total_weight += weight;
                    sum_x += px as f64 * weight;
                    sum_y += py as f64 * weight;
                }

                if total_weight <= 0.0 {
                    rejected_blobs += 1;
                    continue;
                }

                let sub_x = sum_x / total_weight;
                let sub_y = sum_y / total_weight;

                // Compute second moments for elongation and FWHM
                let mut mu_xx = 0.0;
                let mut mu_yy = 0.0;
                let mut mu_xy = 0.0;

                for &(px, py, pval) in &component {
                    let weight = (pval - global_bg_mean).max(0.1);
                    let dx = px as f64 - sub_x;
                    let dy = py as f64 - sub_y;
                    mu_xx += dx * dx * weight;
                    mu_yy += dy * dy * weight;
                    mu_xy += dx * dy * weight;
                }

                mu_xx /= total_weight;
                mu_yy /= total_weight;
                mu_xy /= total_weight;

                let delta = ((mu_xx - mu_yy).powi(2) + 4.0 * mu_xy * mu_xy).sqrt();
                let lambda1 = (mu_xx + mu_yy + delta) / 2.0;
                let lambda2 = ((mu_xx + mu_yy - delta) / 2.0).max(1e-4);
                let elongation = (lambda1 / lambda2).sqrt();

                if elongation > settings.max_elongation {
                    rejected_blobs += 1;
                    continue;
                }

                let fwhm = (2.355 * (lambda1 + lambda2).sqrt() / std::f64::consts::SQRT_2)
                    .clamp(1.0, 10.0);
                let snr = (max_val as f64 - global_bg_mean) / global_bg_std.max(1.0);

                stars.push(DetectedStar {
                    id: stars.len() + 1,
                    x: sub_x,
                    y: sub_y,
                    intensity: total_weight,
                    peak_brightness: max_val,
                    fwhm,
                    snr,
                    elongation,
                });
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
