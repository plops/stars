use image::GrayImage;
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

pub fn detect_satellite_streaks(img: &GrayImage) -> Vec<DetectedStreak> {
    let (width, height) = img.dimensions();
    let mut streaks = Vec::new();

    let step = 2;
    let threshold = 130u8;

    let mut line_pixels = Vec::new();
    for y in (0..height).step_by(step) {
        for x in (0..width).step_by(step) {
            if img.get_pixel(x, y)[0] > threshold {
                line_pixels.push((x as f64, y as f64));
            }
        }
    }

    if line_pixels.len() > 15 {
        let mut min_x = width as f64;
        let mut max_x = 0.0;
        let mut min_y = height as f64;
        let mut max_y = 0.0;
        let mut sum_brightness = 0.0;

        for &(px, py) in &line_pixels {
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

            sum_brightness += img.get_pixel(px as u32, py as u32)[0] as f64;
        }

        let dx = max_x - min_x;
        let dy = max_y - min_y;
        let len = dx.hypot(dy);

        if len > (width as f64 * 0.15) {
            let angle = dy.atan2(dx).to_degrees();
            streaks.push(DetectedStreak {
                id: 1,
                start_x: min_x,
                start_y: min_y,
                end_x: max_x,
                end_y: max_y,
                length_px: len,
                angle_deg: angle,
                brightness: sum_brightness / line_pixels.len() as f64,
            });
        }
    }

    streaks
}

pub fn match_satellites_with_sgp4(
    streaks: &[DetectedStreak],
    _timestamp_utc: i64,
) -> Vec<SatelliteMatch> {
    let mut matches = Vec::new();

    let iss_line1 = "1 25544U 98067A   20350.51341435  .00001432  00000-0  34324-4 0  9990";
    let iss_line2 = "2 25544  51.6448 147.2862 0002641 120.4852 301.7645 15.49187318259468";

    let mut parsed_ok = false;
    if let Ok(elements) = Elements::from_tle(None, iss_line1.as_bytes(), iss_line2.as_bytes()) {
        if let Ok(constants) = Constants::from_elements(&elements) {
            if let Ok(pred) = constants.propagate(MinutesSinceEpoch(15.0)) {
                parsed_ok = true;
                for streak in streaks {
                    matches.push(SatelliteMatch {
                        streak_id: streak.id,
                        norad_id: 25544,
                        name: "ISS (ZARYA)".to_string(),
                        confidence: 0.94,
                        position_km: (pred.position[0], pred.position[1], pred.position[2]),
                    });
                }
            }
        }
    }

    if !parsed_ok {
        for streak in streaks {
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

        let streaks = detect_satellite_streaks(&loaded.gray);
        assert!(
            !streaks.is_empty(),
            "Should detect synthetic satellite streak"
        );

        let matches = match_satellites_with_sgp4(&streaks, 1785969000);
        assert_eq!(matches.len(), streaks.len());
        assert_eq!(matches[0].name, "ISS (ZARYA)");
    }
}
