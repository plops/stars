use stars::aberration::analyze_aberration;
use stars::astrometry::solve_plate;
use stars::exif::ExifMetadata;
use stars::image_loader::{generate_synthetic_image, SyntheticOptions};
use stars::satellites::{detect_satellite_streaks, match_satellites_with_sgp4};
use stars::star_finder::{detect_stars, DetectionSettings};
use stars::validation::validate_exif;
use stars::web::run_full_pipeline;

#[test]
fn test_end_to_end_synthetic_astrophotography_pipeline() {
    let opts = SyntheticOptions::default();
    let loaded = generate_synthetic_image(&opts);

    // 1. Star Finder
    let settings = DetectionSettings::default();
    let detection = detect_stars(&loaded.gray, &settings);
    assert!(!detection.stars.is_empty(), "Star finder must detect stars");
    assert!(detection.stars.len() >= 5);

    // 2. Plate Solving
    let solution = solve_plate(
        &detection.stars,
        48.137,
        11.576,
        180.0,
        1785969000,
        26.0,
        loaded.width,
        loaded.height,
    );
    assert!(solution.solved, "Synthetic plate solve must succeed");
    assert!(!solution.matches.is_empty());

    // 3. EXIF Validation
    let exif = ExifMetadata::dummy_iphone_metadata();
    let validation = validate_exif(&exif, &solution);
    assert!(validation.gps_valid);

    // 4. Aberration Analysis
    let aberration = analyze_aberration(
        &detection.stars,
        &solution,
        loaded.width,
        loaded.height,
        45.0,
    );
    assert!(
        aberration.quality_score > 0.0,
        "Quality score should be positive"
    );

    // 5. Satellite Track Detection
    let streaks = detect_satellite_streaks(&loaded.gray);
    assert!(
        !streaks.is_empty(),
        "Satellite detector must identify linear streak"
    );

    let sat_matches = match_satellites_with_sgp4(&streaks, 1785969000);
    assert_eq!(sat_matches.len(), streaks.len());
    assert_eq!(sat_matches[0].name, "ISS (ZARYA)");
}

#[test]
fn test_full_pipeline_helper() {
    let opts = SyntheticOptions::default();
    let loaded = generate_synthetic_image(&opts);

    let result = run_full_pipeline(&loaded);
    assert_eq!(result.width, 1200);
    assert_eq!(result.height, 900);
    assert!(result.solution.solved);
    assert!(!result.satellite_report.streaks.is_empty());
}
