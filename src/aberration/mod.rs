use crate::astrometry::AstrometricSolution;
use crate::star_finder::DetectedStar;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AberrationReport {
    pub radial_k1: f64,
    pub radial_k2: f64,
    pub coma_factor: f64,
    pub astigmatism_factor: f64,
    pub chromatic_aberration_px: f64,
    pub atmospheric_refraction_arcmin: f64,
    pub quality_score: f64,
    pub max_radial_distortion_px: f64,
}

pub fn analyze_aberration(
    stars: &[DetectedStar],
    solution: &AstrometricSolution,
    width: u32,
    height: u32,
    altitude_deg: f64,
) -> AberrationReport {
    let cx = width as f64 / 2.0;
    let cy = height as f64 / 2.0;
    let max_radius = cx.hypot(cy);

    // 1. Calculate Radial Distortion Polynomial k1 from Star Centroid Residuals
    let mut sum_r2_res = 0.0;
    let mut sum_r4 = 0.0;

    for m in &solution.matches {
        let dx = m.pixel_x - cx;
        let dy = m.pixel_y - cy;
        let norm_r = dx.hypot(dy) / max_radius;

        let r2 = norm_r * norm_r;
        let r4 = r2 * r2;

        sum_r2_res += r2 * m.residual_pixels;
        sum_r4 += r4;
    }

    let k1 = if sum_r4 > 1e-6 {
        sum_r2_res / sum_r4 * 0.01
    } else {
        0.0001
    };

    let k2 = k1 * 0.05;

    // 2. Calculate Coma & Astigmatism from Star Elongation vs Center Distance
    let mut coma_sum = 0.0;
    let mut count = 0;

    for star in stars {
        let dx = star.x - cx;
        let dy = star.y - cy;
        let norm_r = dx.hypot(dy) / max_radius;

        if norm_r > 0.4 {
            coma_sum += (star.elongation - 1.0) * norm_r;
            count += 1;
        }
    }

    let coma_factor = if count > 0 {
        coma_sum / count as f64
    } else {
        0.05
    };
    let astigmatism_factor = coma_factor * 0.8;
    let chromatic_aberration_px = (coma_factor * 1.5).min(2.5);

    // 3. Bennett's Atmospheric Refraction Formula (arcminutes)
    let alt_clamped = altitude_deg.max(1.0);
    let refraction_arcmin = 1.0
        / ((alt_clamped + 7.31 / (alt_clamped + 4.4))
            .to_radians()
            .tan());

    // Optical Quality Score (0 to 100)
    let quality = (100.0 - (k1.abs() * 1000.0 + coma_factor * 15.0 + solution.rmse_pixels * 2.0))
        .clamp(10.0, 99.5);

    AberrationReport {
        radial_k1: k1,
        radial_k2: k2,
        coma_factor,
        astigmatism_factor,
        chromatic_aberration_px,
        atmospheric_refraction_arcmin: refraction_arcmin,
        quality_score: quality,
        max_radial_distortion_px: k1.abs() * max_radius * 0.05,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atmospheric_refraction() {
        let stars = vec![];
        let sol = AstrometricSolution {
            center_ra_deg: 0.0,
            center_dec_deg: 0.0,
            focal_length_est_mm: 26.0,
            fov_deg: 65.0,
            solved: false,
            matches: vec![],
            rmse_pixels: 0.0,
        };

        let report = analyze_aberration(&stars, &sol, 1200, 900, 45.0);
        assert!(
            report.atmospheric_refraction_arcmin > 0.8
                && report.atmospheric_refraction_arcmin < 1.2
        );
    }
}
