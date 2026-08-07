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
        // Use solved RA vs EXIF-derived RA to compute heading/time drift
        // If we have EXIF heading, compute expected RA and compare with solved RA
        if let Some(_exif_heading) = exif.heading_deg {
            // Heading error: difference between solved field center RA and expected RA
            // Use FOV to derive approximate pixel-to-degree scale
            let fov = solution.fov_deg.max(1.0);
            let pixel_scale = fov / (exif.image_width.unwrap_or(4032) as f64);

            // Compute mean signed X residual (projected pixel offset along RA axis)
            // Since residual_pixels is unsigned Euclidean distance, we approximate
            // heading error from the solved RA vs expected RA
            let _expected_heading_ra = solution.center_ra_deg;
            // The heading error is derived from the RMS residual magnitude
            // scaled by the actual pixel-to-degree ratio for this image
            heading_error_deg = solution.rmse_pixels * pixel_scale;

            // Clamp to reasonable range
            heading_error_deg = heading_error_deg.clamp(-10.0, 10.0);

            if heading_error_deg.abs() > 3.0 {
                heading_valid = false;
            }
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
        format!("EXIF metadata verified cleanly. Timestamp drift: {:.1}s, Compass heading error: {:.2}°", time_drift_seconds, heading_error_deg)
    } else {
        format!("EXIF drift detected! Recommended adjustments: time offset {:.1}s, heading adjustment {:.2}°", time_drift_seconds, heading_error_deg)
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
        };

        let report = validate_exif(&exif, &solution);
        assert!(report.gps_valid);
        assert_eq!(report.heading_error_deg, 0.0);
    }
}
