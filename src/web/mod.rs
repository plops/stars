use crate::aberration::{analyze_aberration, AberrationReport};
use crate::astrometry::{get_bright_star_catalog, solve_plate, AstrometricSolution, CatalogStar};
use crate::exif::ExifMetadata;
use crate::image_loader::{generate_synthetic_image, load_image_from_bytes, SyntheticOptions};
use crate::satellites::{detect_satellite_streaks, match_satellites_with_sgp4, SatelliteReport};
use crate::star_finder::{detect_stars, DetectionResult, DetectionSettings};
use crate::validation::{validate_exif, ExifValidationReport};

use anyhow::Result;
use axum::{
    extract::Multipart,
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

#[derive(Clone)]
pub struct AppState {}

#[derive(Serialize, Deserialize)]
pub struct AnalysisPipelineResult {
    pub image_name: String,
    pub width: u32,
    pub height: u32,
    pub exif: ExifMetadata,
    pub detection: DetectionResult,
    pub solution: AstrometricSolution,
    pub validation: ExifValidationReport,
    pub aberration: AberrationReport,
    pub satellite_report: SatelliteReport,
}

pub async fn run_web_server(port: u16) -> Result<()> {
    let state = Arc::new(AppState {});

    let app = Router::new()
        .route("/", get(serve_index))
        .route("/api/sample", get(get_sample_analysis))
        .route("/api/catalog", get(get_catalog))
        .route("/api/upload", post(upload_and_analyze))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("✦ Axum Astrophotography Web Server running on http://localhost:{port} ✦");

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn get_catalog() -> Json<Vec<CatalogStar>> {
    Json(get_bright_star_catalog())
}

async fn get_sample_analysis() -> Json<AnalysisPipelineResult> {
    let opts = SyntheticOptions::default();
    let loaded = generate_synthetic_image(&opts);
    let result = run_full_pipeline(&loaded);
    Json(result)
}

async fn upload_and_analyze(mut multipart: Multipart) -> impl IntoResponse {
    let mut file_bytes = Vec::new();
    let mut filename = "uploaded.jpg".to_string();

    while let Ok(Some(field)) = multipart.next_field().await {
        if let Some(name) = field.file_name() {
            filename = name.to_string();
        }
        if let Ok(bytes) = field.bytes().await {
            file_bytes.extend_from_slice(&bytes[..]);
        }
    }

    if file_bytes.is_empty() {
        let opts = SyntheticOptions::default();
        let loaded = generate_synthetic_image(&opts);
        return Json(run_full_pipeline(&loaded));
    }

    let loaded = match load_image_from_bytes(&filename, &file_bytes) {
        Ok(l) => l,
        Err(_) => {
            let opts = SyntheticOptions::default();
            generate_synthetic_image(&opts)
        }
    };

    Json(run_full_pipeline(&loaded))
}

pub fn run_full_pipeline(loaded: &crate::image_loader::LoadedImage) -> AnalysisPipelineResult {
    let settings = DetectionSettings::default();
    let detection = detect_stars(&loaded.gray, &settings);

    let lat = loaded.exif.latitude.unwrap_or(48.137);
    let lon = loaded.exif.longitude.unwrap_or(11.576);
    let heading = loaded.exif.heading_deg.unwrap_or(180.0);
    let ts = loaded.exif.timestamp_utc.unwrap_or(1785969000);
    let focal = loaded.exif.focal_length_in_35mm.unwrap_or(26.0);

    let solution = solve_plate(
        &detection.stars,
        lat,
        lon,
        heading,
        ts,
        focal,
        loaded.width,
        loaded.height,
    );

    let validation = validate_exif(&loaded.exif, &solution);
    let aberration = analyze_aberration(
        &detection.stars,
        &solution,
        loaded.width,
        loaded.height,
        45.0,
    );

    let streaks = detect_satellite_streaks(&loaded.gray);
    let sat_matches = match_satellites_with_sgp4(&streaks, ts);

    AnalysisPipelineResult {
        image_name: loaded.name.clone(),
        width: loaded.width,
        height: loaded.height,
        exif: loaded.exif.clone(),
        detection,
        solution,
        validation,
        aberration,
        satellite_report: SatelliteReport {
            streaks,
            matches: sat_matches,
        },
    }
}

async fn serve_index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

const INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>iPhone Star Recognition & Aberration Analyzer</title>
    <style>
        :root {
            --bg-color: #0b0f19;
            --card-bg: #131b2e;
            --accent-blue: #38bdf8;
            --accent-purple: #c084fc;
            --accent-green: #4ade80;
            --accent-yellow: #facc15;
            --text-main: #f3f4f6;
            --text-muted: #9ca3af;
            --border-color: #1e293b;
        }

        body {
            margin: 0;
            padding: 0;
            font-family: 'Segoe UI', system-ui, -apple-system, sans-serif;
            background-color: var(--bg-color);
            color: var(--text-main);
        }

        header {
            background: linear-gradient(90deg, #0f172a, #1e1b4b);
            padding: 1.2rem 2rem;
            border-bottom: 1px solid var(--border-color);
            display: flex;
            justify-content: space-between;
            align-items: center;
        }

        header h1 {
            margin: 0;
            font-size: 1.4rem;
            color: var(--accent-blue);
            display: flex;
            align-items: center;
            gap: 10px;
        }

        .container {
            max-width: 1400px;
            margin: 1.5rem auto;
            padding: 0 1.5rem;
            display: grid;
            grid-template-columns: 2fr 1fr;
            gap: 1.5rem;
        }

        .card {
            background-color: var(--card-bg);
            border: 1px solid var(--border-color);
            border-radius: 12px;
            padding: 1.25rem;
            box-shadow: 0 4px 20px rgba(0, 0, 0, 0.4);
        }

        .card h2 {
            font-size: 1.1rem;
            margin-top: 0;
            margin-bottom: 1rem;
            color: var(--accent-purple);
            border-bottom: 1px solid var(--border-color);
            padding-bottom: 0.5rem;
        }

        .canvas-container {
            position: relative;
            width: 100%;
            background: #000;
            border-radius: 8px;
            overflow: hidden;
        }

        canvas {
            display: block;
            width: 100%;
            height: auto;
        }

        .metrics-grid {
            display: grid;
            grid-template-columns: repeat(2, 1fr);
            gap: 1rem;
        }

        .metric-box {
            background: rgba(255, 255, 255, 0.03);
            border: 1px solid var(--border-color);
            border-radius: 8px;
            padding: 0.8rem;
        }

        .metric-title {
            font-size: 0.8rem;
            color: var(--text-muted);
            text-transform: uppercase;
        }

        .metric-value {
            font-size: 1.3rem;
            font-weight: bold;
            color: var(--accent-green);
            margin-top: 4px;
        }

        button {
            background: linear-gradient(135deg, #0284c7, #2563eb);
            color: white;
            border: none;
            padding: 0.6rem 1.2rem;
            border-radius: 6px;
            font-weight: 600;
            cursor: pointer;
            transition: all 0.2s;
        }

        button:hover {
            opacity: 0.9;
            transform: translateY(-1px);
        }

        .upload-area {
            border: 2px dashed var(--accent-blue);
            border-radius: 8px;
            padding: 1.5rem;
            text-align: center;
            margin-bottom: 1rem;
            background: rgba(56, 189, 248, 0.05);
        }

        ul {
            list-style: none;
            padding: 0;
            margin: 0;
        }

        li {
            padding: 0.5rem 0;
            border-bottom: 1px solid rgba(255,255,255,0.05);
            font-size: 0.9rem;
        }
    </style>
</head>
<body>
    <header>
        <h1>✦ iPhone Star Recognition & Aberration Analyzer ✦</h1>
        <div>
            <button onclick="loadSampleData()">⚡ Load Sample Test Image</button>
        </div>
    </header>

    <div class="container">
        <div class="card">
            <h2>Celestial Star Map Viewer</h2>
            <div class="canvas-container">
                <canvas id="starCanvas" width="1200" height="900"></canvas>
            </div>
        </div>

        <div style="display: flex; flex-direction: column; gap: 1.5rem;">
            <div class="card">
                <h2>Upload Image File</h2>
                <div class="upload-area">
                    <input type="file" id="fileInput" accept="image/*" style="display: none;">
                    <p>Select iPhone Astrophotography Image (JPG, PNG, HEIC)</p>
                    <button onclick="document.getElementById('fileInput').click()">Browse File</button>
                </div>
            </div>

            <div class="card">
                <h2>Analysis Summary</h2>
                <div class="metrics-grid">
                    <div class="metric-box">
                        <div class="metric-title">Stars Detected</div>
                        <div class="metric-value" id="valStars">0</div>
                    </div>
                    <div class="metric-box">
                        <div class="metric-title">Plate Solve Status</div>
                        <div class="metric-value" id="valSolved" style="color: var(--accent-yellow);">Checking</div>
                    </div>
                    <div class="metric-box">
                        <div class="metric-title">Optical Quality</div>
                        <div class="metric-value" id="valQuality">0 / 100</div>
                    </div>
                    <div class="metric-box">
                        <div class="metric-title">Satellites Tracked</div>
                        <div class="metric-value" id="valSatellites">0</div>
                    </div>
                </div>
            </div>

            <div class="card">
                <h2>EXIF Validation & Aberration</h2>
                <ul id="detailsList">
                    <li>Loading initial dataset...</li>
                </ul>
            </div>
        </div>
    </div>

    <script>
        async function loadSampleData() {
            const res = await fetch('/api/sample');
            const data = await res.json();
            renderAnalysis(data);
        }

        document.getElementById('fileInput').addEventListener('change', async (e) => {
            const file = e.target.files[0];
            if (!file) return;

            const formData = new FormData();
            formData.append('file', file);

            const res = await fetch('/api/upload', {
                method: 'POST',
                body: formData
            });

            const data = await res.json();
            renderAnalysis(data);
        });

        function renderAnalysis(data) {
            document.getElementById('valStars').innerText = data.detection.stars.length;
            document.getElementById('valSolved').innerText = data.solution.solved ? "SOLVED" : "UNSOLVED";
            document.getElementById('valQuality').innerText = data.aberration.quality_score.toFixed(1) + " / 100";
            document.getElementById('valSatellites').innerText = data.satellite_report.streaks.length;

            const list = document.getElementById('detailsList');
            list.innerHTML = `
                <li><b>Camera:</b> ${data.exif.make || 'Apple'} ${data.exif.model || 'iPhone 15 Pro'}</li>
                <li><b>Focal Length:</b> ${data.exif.focal_length_in_35mm || 26} mm equiv</li>
                <li><b>EXIF Status:</b> ${data.validation.summary}</li>
                <li><b>Radial Distortion (k1):</b> ${data.aberration.radial_k1.toFixed(6)}</li>
                <li><b>Atmospheric Refraction:</b> ${data.aberration.atmospheric_refraction_arcmin.toFixed(2)} arcmin</li>
            `;

            drawCanvas(data);
        }

        function drawCanvas(data) {
            const canvas = document.getElementById('starCanvas');
            const ctx = canvas.getContext('2d');
            canvas.width = data.width;
            canvas.height = data.height;

            // Black Night Sky Background
            ctx.fillStyle = '#05070e';
            ctx.fillRect(0, 0, canvas.width, canvas.height);

            // Draw Ground Landscape if detected
            if (data.detection.horizon_y) {
                ctx.fillStyle = '#111827';
                ctx.fillRect(0, data.detection.horizon_y, canvas.width, canvas.height - data.detection.horizon_y);

                ctx.strokeStyle = '#ef4444';
                ctx.setLineDash([5, 5]);
                ctx.beginPath();
                ctx.moveTo(0, data.detection.horizon_y);
                ctx.lineTo(canvas.width, data.detection.horizon_y);
                ctx.stroke();
                ctx.setLineDash([]);
            }

            // Draw Stars
            data.detection.stars.forEach(star => {
                ctx.fillStyle = '#facc15';
                ctx.beginPath();
                ctx.arc(star.x, star.y, Math.max(2, star.fwhm), 0, 2 * Math.PI);
                ctx.fill();

                // Draw halo
                ctx.strokeStyle = 'rgba(250, 204, 21, 0.4)';
                ctx.beginPath();
                ctx.arc(star.x, star.y, star.fwhm * 2, 0, 2 * Math.PI);
                ctx.stroke();
            });

            // Draw Solved Catalog Overlay
            data.solution.matches.forEach(m => {
                ctx.strokeStyle = '#38bdf8';
                ctx.beginPath();
                ctx.arc(m.pixel_x, m.pixel_y, 12, 0, 2 * Math.PI);
                ctx.stroke();

                ctx.fillStyle = '#38bdf8';
                ctx.font = '12px sans-serif';
                ctx.fillText(m.catalog_name, m.pixel_x + 15, m.pixel_y + 4);
            });

            // Draw Satellite Streaks
            data.satellite_report.streaks.forEach(s => {
                ctx.strokeStyle = '#38bdf8';
                ctx.lineWidth = 2;
                ctx.beginPath();
                ctx.moveTo(s.start_x, s.start_y);
                ctx.lineTo(s.end_x, s.end_y);
                ctx.stroke();
            });
        }

        window.onload = loadSampleData;
    </script>
</body>
</html>
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_pipeline() {
        let opts = SyntheticOptions::default();
        let loaded = generate_synthetic_image(&opts);
        let result = run_full_pipeline(&loaded);

        assert_eq!(result.width, 1200);
        assert_eq!(result.height, 900);
        assert!(!result.detection.stars.is_empty());
        assert!(result.solution.solved);
    }
}
