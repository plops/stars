use anyhow::{Context, Result};
use clap::Parser;
use stars::image_loader::{
    generate_synthetic_image, load_image_from_path, LoadedImage, SyntheticOptions,
};
use stars::tui::{run_tui, TuiAppState};
use stars::web::{run_full_pipeline, run_web_server};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "stars",
    author = "wol pumba <wolpumba@gmail.com>",
    version = "0.1.0",
    about = "iPhone Star Recognition, Astrometry, EXIF Validation, Aberration & Satellite Tracker"
)]
struct Args {
    /// Input image file path
    #[arg(short, long)]
    image: Option<PathBuf>,

    /// Directory containing image sequence
    #[arg(short, long)]
    sequence: Option<PathBuf>,

    /// Launch Ratatui Terminal UI mode
    #[arg(short, long)]
    tui: bool,

    /// Launch Axum Web Application Server mode
    #[arg(short, long)]
    web: bool,

    /// Web server listening port
    #[arg(short, long, default_value_t = 5001)]
    port: u16,

    /// Export analysis JSON report to file path
    #[arg(short, long)]
    export_json: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // 1. Launch Web Server if requested
    if args.web {
        return run_web_server(args.port).await;
    }

    // 2. Load or Generate Input Image
    let loaded_img: LoadedImage = match &args.image {
        Some(path) => load_image_from_path(path)
            .with_context(|| format!("Failed to load image from path {}", path.display()))?,
        None => {
            println!("✦ No input image specified; generating synthetic iPhone test image with stars & satellites... ✦");
            let opts = SyntheticOptions::default();
            generate_synthetic_image(&opts)
        }
    };

    // 3. Execute Analysis Pipeline
    let result = run_full_pipeline(&loaded_img);

    // 4. Export JSON Report if specified
    if let Some(json_path) = &args.export_json {
        let json_str = serde_json::to_string_pretty(&result)?;
        std::fs::write(json_path, json_str)
            .with_context(|| format!("Failed to export JSON report to {}", json_path.display()))?;
        println!(
            "✦ Exported JSON analysis report to {} ✦",
            json_path.display()
        );
    }

    // 5. Launch TUI mode or Print CLI Summary
    if args.tui {
        let app_state = TuiAppState {
            active_tab: 0,
            image_name: result.image_name,
            image_width: result.width,
            image_height: result.height,
            exif: result.exif,
            detection: result.detection,
            solution: result.solution,
            aberration: result.aberration,
            satellite_report: result.satellite_report,
        };
        run_tui(app_state)?;
    } else {
        println!("\n========================================================");
        println!("✦ iPHONE STAR RECOGNITION & ABERRATION ANALYSIS ✦");
        println!("========================================================");
        println!("Image Name:           {}", result.image_name);
        println!(
            "Resolution:           {}x{} px",
            result.width, result.height
        );
        println!(
            "Camera Model:         {}",
            result.exif.model.as_deref().unwrap_or("Unknown")
        );
        println!("Detected Stars:       {}", result.detection.stars.len());
        println!("Landscape Horizon Y:  {:?}", result.detection.horizon_y);
        println!(
            "Plate Solve Status:   {}",
            if result.solution.solved {
                "SUCCESS"
            } else {
                "UNSOLVED"
            }
        );
        println!("Matched Catalog Stars: {}", result.solution.matches.len());
        println!(
            "RMS Residual Error:   {:.2} px",
            result.solution.rmse_pixels
        );
        println!("EXIF Validation:      {}", result.validation.summary);
        println!("Radial Distortion k1: {:.6}", result.aberration.radial_k1);
        println!(
            "Atmospheric Refraction: {:.2} arcmin",
            result.aberration.atmospheric_refraction_arcmin
        );
        println!(
            "Optical Quality Score: {:.1} / 100",
            result.aberration.quality_score
        );
        println!(
            "Detected Satellites:  {}",
            result.satellite_report.streaks.len()
        );
        for sat in &result.satellite_report.matches {
            println!(
                "   ↳ Matched Satellite: {} (NORAD {})",
                sat.name, sat.norad_id
            );
        }
        println!("========================================================\n");
        println!("Tip: Run with --tui for terminal dashboard or --web for web viewer!");
    }

    Ok(())
}
