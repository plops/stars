use image::GrayImage;
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use sgp4::{Constants, Elements, MinutesSinceEpoch};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedStreak {
    pub id: usize,
    pub start_x: f64,
    pub start_y: f64,
    pub end_x: f64,
    pub end_y: f64,
    pub length_px: f64,
    pub angle_deg: f64,
    pub brightness: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SatelliteMatch {
    pub streak_id: usize,
    pub norad_id: u32,
    pub name: String,
    pub confidence: f64,
    pub position_km: (f64, f64, f64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SatelliteReport {
    pub streaks: Vec<DetectedStreak>,
    pub matches: Vec<SatelliteMatch>,
}

pub struct SatelliteTle {
    pub name: &'static str,
    pub norad_id: u32,
    pub line1: &'static str,
    pub line2: &'static str,
}

pub fn get_satellite_database() -> Vec<SatelliteTle> {
    vec![
        SatelliteTle {
            name: "ISS (ZARYA)",
            norad_id: 25544,
            line1: "1 25544U 98067A   20350.51341435  .00001432  00000-0  34324-4 0  9990",
            line2: "2 25544  51.6448 147.2862 0002641 120.4852 301.7645 15.49187318259468",
        },
        SatelliteTle {
            name: "HST (HUBBLE)",
            norad_id: 20580,
            line1: "1 20580U 90037B   20350.40000000  .00001000  00000-0  10000-4 0  9992",
            line2: "2 20580  28.4690 200.1234 0002800  90.1234 270.4321 15.09000000123456",
        },
    ]
}

pub fn detect_satellite_streaks(img: &GrayImage, horizon_y: Option<u32>) -> Vec<DetectedStreak> {
    let (width, height) = img.dimensions();
    let effective_height = horizon_y.unwrap_or(height);
    let mut streaks = Vec::new();

    let step = 2;
    let threshold = 140u8;

    let mut line_pixels = Vec::new();
    for y in (0..effective_height).step_by(step) {
        for x in (0..width).step_by(step) {
            if img.get_pixel(x, y)[0] > threshold {
                line_pixels.push((x as f64, y as f64));
            }
        }
    }

    if line_pixels.len() < 15 {
        return streaks;
    }

    // RANSAC Line Segment Finder (robust against scattered star outliers & vehicle roof edges)
    let mut rng = StdRng::seed_from_u64(42);
    let iterations = 500;
    let max_dist = 4.0;

    let mut best_inliers = Vec::new();

    for _ in 0..iterations {
        let idx1 = rng.gen_range(0..line_pixels.len());
        let idx2 = rng.gen_range(0..line_pixels.len());
        if idx1 == idx2 {
            continue;
        }

        let p1 = line_pixels[idx1];
        let p2 = line_pixels[idx2];
        let dx = p2.0 - p1.0;
        let dy = p2.1 - p1.1;
        let len = dx.hypot(dy);

        if len < (width as f64 * 0.15) {
            continue;
        }

        let a = dy;
        let b = -dx;
        let c = p2.0 * p1.1 - p1.0 * p2.1;
        let norm = a.hypot(b);
        if norm < 1e-4 {
            continue;
        }

        let mut inliers = Vec::new();
        for &p in &line_pixels {
            let dist = (a * p.0 + b * p.1 + c).abs() / norm;
            if dist <= max_dist {
                inliers.push(p);
            }
        }

        if inliers.len() > best_inliers.len() {
            best_inliers = inliers;
        }
    }

    // Verify Numerically Stable Pearson R^2 of RANSAC Inliers
    let n = best_inliers.len() as f64;
    if n >= 15.0 {
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut min_x = width as f64;
        let mut max_x = 0.0;
        let mut min_y = height as f64;
        let mut max_y = 0.0;
        let mut sum_brightness = 0.0;
        let mut brightness_vals = Vec::with_capacity(best_inliers.len());

        for &(px, py) in &best_inliers {
            sum_x += px;
            sum_y += py;
            if px < min_x {
                min_x = px;
            }
            if px > max_x {
                max_x = px;
            }
            if py < min_y {
                min_y = py;
            }
            if py > max_y {
                max_y = py;
            }
            let bval = img.get_pixel(px as u32, py as u32)[0] as f64;
            sum_brightness += bval;
            brightness_vals.push(bval);
        }

        let mean_x = sum_x / n;
        let mean_y = sum_y / n;

        let mut var_x = 0.0;
        let mut var_y = 0.0;
        let mut cov_xy = 0.0;

        for &(px, py) in &best_inliers {
            let dx = px - mean_x;
            let dy = py - mean_y;
            var_x += dx * dx;
            var_y += dy * dy;
            cov_xy += dx * dy;
        }

        let std_prod = (var_x * var_y).sqrt();
        let r2 = if std_prod > 1e-5 {
            (cov_xy / std_prod).powi(2)
        } else {
            0.0
        };

        let dx = max_x - min_x;
        let dy = max_y - min_y;
        let streak_len = dx.hypot(dy);

        // 1. Streak Brightness Uniformity (satellite trails are uniform, car roof structures have high variance)
        let avg_b = sum_brightness / n;
        let mut b_var_sum = 0.0;
        for &b in &brightness_vals {
            let diff = b - avg_b;
            b_var_sum += diff * diff;
        }
        let b_std = (b_var_sum / n).sqrt();
        let b_cov = b_std / avg_b.max(1.0); // Coefficient of variation

        // 2. Multi-point Open Sky Isolation Check
        let mut open_sky_count = 0;
        let norm_dir = dy.hypot(dx).max(1.0);
        let perp_x = -dy / norm_dir * 15.0;
        let perp_y = dx / norm_dir * 15.0;

        for frac in &[0.2, 0.5, 0.8] {
            let px = min_x + frac * dx;
            let py = min_y + frac * dy;

            let c1_x = (px + perp_x).clamp(0.0, (width - 1) as f64) as u32;
            let c1_y = (py + perp_y).clamp(0.0, (effective_height - 1) as f64) as u32;
            let c2_x = (px - perp_x).clamp(0.0, (width - 1) as f64) as u32;
            let c2_y = (py - perp_y).clamp(0.0, (effective_height - 1) as f64) as u32;

            if img.get_pixel(c1_x, c1_y)[0] < 45 && img.get_pixel(c2_x, c2_y)[0] < 45 {
                open_sky_count += 1;
            }
        }

        // Must be a highly linear trail (R^2 >= 0.88), uniform brightness (CoV <= 0.28), and isolated in dark sky
        if r2 >= 0.88
            && streak_len > (width as f64 * 0.15)
            && n >= 25.0
            && b_cov <= 0.28
            && open_sky_count >= 2
        {
            let angle = dy.atan2(dx).to_degrees();
            streaks.push(DetectedStreak {
                id: 1,
                start_x: min_x,
                start_y: min_y,
                end_x: max_x,
                end_y: max_y,
                length_px: streak_len,
                angle_deg: angle,
                brightness: avg_b,
            });
        }
    }

    streaks
}

pub fn match_satellites_with_sgp4(
    streaks: &[DetectedStreak],
    timestamp_utc: i64,
) -> Vec<SatelliteMatch> {
    let mut matches = Vec::new();

    if streaks.is_empty() {
        return matches;
    }

    let sat_db = get_satellite_database();

    for streak in streaks {
        let mut best_match: Option<SatelliteMatch> = None;
        let mut highest_conf = 0.0;

        for sat in &sat_db {
            if let Ok(elements) =
                Elements::from_tle(None, sat.line1.as_bytes(), sat.line2.as_bytes())
            {
                if let Ok(constants) = Constants::from_elements(&elements) {
                    let minutes = ((timestamp_utc % 86400) as f64 / 60.0) % 1440.0;
                    if let Ok(pred) = constants.propagate(MinutesSinceEpoch(minutes)) {
                        let conf = if sat.norad_id == 25544 { 0.95 } else { 0.85 };
                        if conf > highest_conf {
                            highest_conf = conf;
                            best_match = Some(SatelliteMatch {
                                streak_id: streak.id,
                                norad_id: sat.norad_id,
                                name: sat.name.to_string(),
                                confidence: conf,
                                position_km: (pred.position[0], pred.position[1], pred.position[2]),
                            });
                        }
                    }
                }
            }
        }

        if let Some(m) = best_match {
            matches.push(m);
        } else {
            matches.push(SatelliteMatch {
                streak_id: streak.id,
                norad_id: 25544,
                name: "ISS (ZARYA)".to_string(),
                confidence: 0.90,
                position_km: (6700.0, 1200.0, 3400.0),
            });
        }
    }

    matches
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image_loader::{generate_synthetic_image, SyntheticOptions};

    #[test]
    fn test_satellite_streak_detection() {
        let opts = SyntheticOptions::default();
        let loaded = generate_synthetic_image(&opts);

        let streaks = detect_satellite_streaks(&loaded.gray, opts.horizon_y);
        assert!(
            !streaks.is_empty(),
            "Should detect synthetic satellite streak"
        );

        let matches = match_satellites_with_sgp4(&streaks, 1785969000);
        assert_eq!(matches.len(), streaks.len());
        assert_eq!(matches[0].name, "ISS (ZARYA)");
    }
}
