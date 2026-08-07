use crate::astrometry::AstrometricSolution;
use crate::star_finder::DetectedStar;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AberrationReport {
    pub radial_k1: f64,
    pub radial_k2: f64,
    pub radial_fit_rmse_px: f64,
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
    rgb_image: Option<&image::RgbImage>,
) -> AberrationReport {
    let cx = width as f64 / 2.0;
    let cy = height as f64 / 2.0;
    let max_radius = cx.hypot(cy);

    // 1. Least-Squares Polynomial Fit for Radial Distortion Coefficients k1 & k2
    // Model: Delta_r = k1 * r^3 + k2 * r^5
    let mut sum_r6 = 0.0;
    let mut sum_r8 = 0.0;
    let mut sum_r10 = 0.0;
    let mut sum_r3_dr = 0.0;
    let mut sum_r5_dr = 0.0;

    for m in &solution.matches {
        let dx = m.pixel_x - cx;
        let dy = m.pixel_y - cy;
        let norm_r = dx.hypot(dy) / max_radius;

        let dr = m.residual_pixels / max_radius;
        let r3 = norm_r.powi(3);
        let r5 = norm_r.powi(5);

        sum_r6 += r3 * r3;
        sum_r8 += r3 * r5;
        sum_r10 += r5 * r5;
        sum_r3_dr += r3 * dr;
        sum_r5_dr += r5 * dr;
    }

    let det = sum_r6 * sum_r10 - sum_r8 * sum_r8;
    let (k1, k2) = if det.abs() > 1e-12 {
        let k1_val = (sum_r10 * sum_r3_dr - sum_r8 * sum_r5_dr) / det;
        let k2_val = (sum_r6 * sum_r5_dr - sum_r8 * sum_r3_dr) / det;
        (k1_val.clamp(-0.05, 0.05), k2_val.clamp(-0.05, 0.05))
    } else if sum_r6 > 1e-8 {
        (sum_r3_dr / sum_r6, 0.0001)
    } else {
        (0.0001, 0.00001)
    };

    // 2. Measure Coma & Astigmatism from Star Elongation Gradient
    let mut coma_sum = 0.0;
    let mut astig_sum = 0.0;
    let mut edge_count = 0;

    for star in stars {
        let dx = star.x - cx;
        let dy = star.y - cy;
        let norm_r = dx.hypot(dy) / max_radius;

        if norm_r > 0.35 {
            let radial_elongation = (star.elongation - 1.0) * norm_r;
            coma_sum += radial_elongation;
            astig_sum += radial_elongation * (1.0 - norm_r);
            edge_count += 1;
        }
    }

    let coma_factor = if edge_count > 0 {
        coma_sum / edge_count as f64
    } else {
        0.0
    };

    let astigmatism_factor = if edge_count > 0 {
        astig_sum / edge_count as f64
    } else {
        0.0
    };

    // Measure chromatic aberration from RGB channel centroid displacement near edges
    let chromatic_aberration_px = if let Some(rgb) = rgb_image {
        measure_rgb_chromatic_aberration(rgb, stars, cx, cy, max_radius)
    } else {
        // Fallback estimate when no RGB data available
        (coma_factor.abs() * 1.2 + k1.abs() * 3.0).clamp(0.0, 3.5)
    };

    // 3. Bennett's Atmospheric Refraction Formula (arcminutes)
    let refraction_arcmin = atmospheric_refraction_correction(altitude_deg) * 60.0;

    // Optical Quality Score (0 to 100)
    let quality = (100.0
        - (k1.abs() * 1200.0
            + k2.abs() * 2400.0
            + coma_factor * 18.0
            + solution.rmse_pixels * 2.5))
        .clamp(10.0, 99.5);

    // Compute radial distortion model fit RMSE
    let radial_fit_rmse_px = if !solution.matches.is_empty() {
        let sq_err: f64 = solution
            .matches
            .iter()
            .map(|m| {
                let dx = m.pixel_x - cx;
                let dy = m.pixel_y - cy;
                let norm_r = dx.hypot(dy) / max_radius;
                let dr_model = (k1 * norm_r.powi(3) + k2 * norm_r.powi(5)) * max_radius;
                (m.residual_pixels - dr_model).powi(2)
            })
            .sum();
        (sq_err / solution.matches.len() as f64).sqrt()
    } else {
        0.0
    };

    AberrationReport {
        radial_k1: k1,
        radial_k2: k2,
        radial_fit_rmse_px,
        coma_factor,
        astigmatism_factor,
        chromatic_aberration_px,
        atmospheric_refraction_arcmin: refraction_arcmin,
        quality_score: quality,
        max_radial_distortion_px: (k1.abs() + k2.abs()) * max_radius,
    }
}

/// Measure chromatic aberration by comparing R vs B channel centroids near image edges
fn measure_rgb_chromatic_aberration(
    rgb: &image::RgbImage,
    stars: &[DetectedStar],
    cx: f64,
    cy: f64,
    max_radius: f64,
) -> f64 {
    let (w, h) = rgb.dimensions();
    let mut total_shift = 0.0;
    let mut count = 0;

    for star in stars {
        let norm_r = ((star.x - cx).powi(2) + (star.y - cy).powi(2)).sqrt() / max_radius;
        // Only measure near edges (outer 40% of image radius)
        if norm_r < 0.6 {
            continue;
        }

        let sx = star.x.round() as i32;
        let sy = star.y.round() as i32;
        let half = 4i32; // 9x9 window

        let mut r_wx = 0.0;
        let mut r_wy = 0.0;
        let mut r_wt = 0.0;
        let mut b_wx = 0.0;
        let mut b_wy = 0.0;
        let mut b_wt = 0.0;

        for dy in -half..=half {
            for dx in -half..=half {
                let px = sx + dx;
                let py = sy + dy;
                if px < 0 || py < 0 || px >= w as i32 || py >= h as i32 {
                    continue;
                }
                let pixel = rgb.get_pixel(px as u32, py as u32);
                let r_val = pixel[0] as f64;
                let b_val = pixel[2] as f64;

                r_wx += px as f64 * r_val;
                r_wy += py as f64 * r_val;
                r_wt += r_val;
                b_wx += px as f64 * b_val;
                b_wy += py as f64 * b_val;
                b_wt += b_val;
            }
        }

        if r_wt > 0.0 && b_wt > 0.0 {
            let r_cx = r_wx / r_wt;
            let r_cy = r_wy / r_wt;
            let b_cx = b_wx / b_wt;
            let b_cy = b_wy / b_wt;
            let shift = ((r_cx - b_cx).powi(2) + (r_cy - b_cy).powi(2)).sqrt();
            total_shift += shift;
            count += 1;
        }
    }

    if count > 0 {
        (total_shift / count as f64).clamp(0.0, 5.0)
    } else {
        0.0
    }
}

/// Bennett's Atmospheric Refraction Formula returning correction in DEGREES
pub fn atmospheric_refraction_correction(alt_deg: f64) -> f64 {
    if alt_deg < 0.0 {
        return 0.0;
    }
    let alt_clamped = alt_deg.max(0.0);
    let term_deg = alt_clamped + 7.31 / (alt_clamped + 4.4);
    let r_arcmin = 1.0 / term_deg.to_radians().tan();
    (r_arcmin / 60.0).max(0.0)
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
            estimated_alt_deg: 45.0,
            focal_length_est_mm: 26.0,
            fov_deg: 65.0,
            solved: false,
            matches: vec![],
            rmse_pixels: 0.0,
            sip_distortion: None,
        };

        let report = analyze_aberration(&stars, &sol, 1200, 900, 45.0, None);
        assert!(
            report.atmospheric_refraction_arcmin > 0.8
                && report.atmospheric_refraction_arcmin < 1.2
        );
        assert!(report.quality_score > 0.0);
    }

    #[test]
    fn test_refraction_at_horizon() {
        let refr_arcmin = atmospheric_refraction_correction(0.0) * 60.0;
        assert!(
            (refr_arcmin - 34.0).abs() < 5.0,
            "Atmospheric refraction at horizon should be ~34 arcmin, got {refr_arcmin}"
        );
    }

    #[test]
    fn test_refraction_at_zenith() {
        let refr_arcmin = atmospheric_refraction_correction(90.0) * 60.0;
        assert!(
            refr_arcmin.abs() < 0.1,
            "Atmospheric refraction at zenith should be ~0 arcmin, got {refr_arcmin}"
        );
    }
}
