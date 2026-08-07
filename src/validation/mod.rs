use crate::astrometry::AstrometricSolution;
use crate::exif::ExifMetadata;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExifValidationReport {
    pub timestamp_valid: bool,
    pub time_drift_seconds: f64,
    pub heading_valid: bool,
    pub heading_error_deg: f64,
    pub gps_valid: bool,
    pub suggested_corrected_timestamp: Option<i64>,
    pub suggested_corrected_heading: Option<f64>,
    pub summary: String,
}

pub fn validate_exif(exif: &ExifMetadata, solution: &AstrometricSolution) -> ExifValidationReport {
    let mut time_drift_seconds = 0.0;
    let mut heading_error_deg = 0.0;
    let mut timestamp_valid = true;
    let mut heading_valid = true;
    let gps_valid = exif.latitude.is_some() && exif.longitude.is_some();

    if solution.solved && !solution.matches.is_empty() {
        let fov = solution.fov_deg.max(1.0);
        let img_w = exif.image_width.unwrap_or(4032) as f64;
        let pixel_scale = fov / img_w;

        let mean_dx: f64 = solution.matches.iter().map(|m| m.dx_pixels).sum::<f64>()
            / solution.matches.len() as f64;

        // Systematic heading error derived from mean signed X-residual
        heading_error_deg = (mean_dx * pixel_scale).clamp(-10.0, 10.0);

        if heading_error_deg.abs() > 3.0 {
            heading_valid = false;
        }

        // Earth rotates at ~15 degrees per hour = 1 degree per 240 seconds
        time_drift_seconds = heading_error_deg * 240.0;
        if time_drift_seconds.abs() > 15.0 {
            timestamp_valid = false;
        }
    }

    let raw_heading = exif.heading_deg.unwrap_or(180.0);
    let corrected_heading = (raw_heading + heading_error_deg + 360.0) % 360.0;

    let raw_time = exif.timestamp_utc.unwrap_or(0);
    let corrected_time = raw_time + time_drift_seconds.round() as i64;

    let summary = if timestamp_valid && heading_valid {
        format!(
            "EXIF metadata verified cleanly. Timestamp drift: {:.1}s, Compass heading error: {:.2}°",
            time_drift_seconds, heading_error_deg
        )
    } else {
        format!(
            "EXIF drift detected! Recommended adjustments: time offset {:.1}s, heading adjustment {:.2}°",
            time_drift_seconds, heading_error_deg
        )
    };

    ExifValidationReport {
        timestamp_valid,
        time_drift_seconds,
        heading_valid,
        heading_error_deg,
        gps_valid,
        suggested_corrected_timestamp: Some(corrected_time),
        suggested_corrected_heading: Some(corrected_heading),
        summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_exif() {
        let exif = ExifMetadata::dummy_iphone_metadata();
        let solution = AstrometricSolution {
            center_ra_deg: 88.0,
            center_dec_deg: 7.0,
            estimated_alt_deg: 45.0,
            focal_length_est_mm: 26.0,
            fov_deg: 65.0,
            solved: true,
            matches: vec![],
            rmse_pixels: 0.5,
            sip_distortion: None,
        };

        let report = validate_exif(&exif, &solution);
        assert!(report.gps_valid);
        assert_eq!(report.heading_error_deg, 0.0);
    }

    #[test]
    fn test_signed_residuals() {
        use crate::astrometry::StarMatch;

        let exif = ExifMetadata {
            heading_deg: Some(180.0),
            image_width: Some(1000),
            image_height: Some(1000),
            ..ExifMetadata::dummy_iphone_metadata()
        };

        let matches = vec![
            StarMatch {
                star_id: 1,
                pixel_x: 533.333,
                pixel_y: 500.0,
                catalog_name: "Star 1".into(),
                catalog_ra: 100.0,
                catalog_dec: 0.0,
                catalog_vmag: 2.0,
                residual_pixels: 33.333,
                dx_pixels: 33.333,
                dy_pixels: 0.0,
            },
            StarMatch {
                star_id: 2,
                pixel_x: 633.333,
                pixel_y: 400.0,
                catalog_name: "Star 2".into(),
                catalog_ra: 105.0,
                catalog_dec: 2.0,
                catalog_vmag: 2.5,
                residual_pixels: 33.333,
                dx_pixels: 33.333,
                dy_pixels: 0.0,
            },
        ];

        let solution = AstrometricSolution {
            center_ra_deg: 100.0,
            center_dec_deg: 0.0,
            estimated_alt_deg: 45.0,
            focal_length_est_mm: 26.0,
            fov_deg: 60.0,
            solved: true,
            matches,
            rmse_pixels: 33.333,
            sip_distortion: None,
        };

        let report = validate_exif(&exif, &solution);
        assert!(
            (report.heading_error_deg - 2.0).abs() < 0.1,
            "Expected heading error ~2.0°, got {}",
            report.heading_error_deg
        );
        assert!(
            (report.time_drift_seconds - 480.0).abs() < 10.0,
            "Expected time drift ~480s, got {}",
            report.time_drift_seconds
        );
    }
}
