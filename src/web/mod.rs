use crate::aberration::{analyze_aberration, AberrationReport};
use crate::astrometry::{get_bright_star_catalog, solve_plate, AstrometricSolution, CatalogStar};
use crate::exif::ExifMetadata;
use crate::image_loader::{generate_synthetic_image, load_image_from_bytes, SyntheticOptions};
use crate::satellites::{detect_satellite_streaks, match_satellites_with_sgp4, SatelliteReport};
use crate::star_finder::{detect_stars, DetectionResult, DetectionSettings};
use crate::validation::{validate_exif, ExifValidationReport};

use anyhow::Result;
use axum::{
    extract::{DefaultBodyLimit, Multipart, Query},
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use base64::Engine;
use image::ImageFormat;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

#[derive(Clone)]
pub struct AppState {}

#[derive(Deserialize)]
pub struct SampleQuery {
    pub sigma: Option<f64>,
}

#[derive(Serialize, Deserialize)]
pub struct AnalysisPipelineResult {
    pub image_name: String,
    pub image_data_url: String,
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
        .layer(DefaultBodyLimit::max(50 * 1024 * 1024))
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

async fn get_sample_analysis(Query(query): Query<SampleQuery>) -> Json<AnalysisPipelineResult> {
    let opts = SyntheticOptions::default();
    let loaded = generate_synthetic_image(&opts);
    let sigma = query.sigma.unwrap_or(2.2);
    let result = run_full_pipeline_with_sigma(&loaded, sigma);
    Json(result)
}

async fn upload_and_analyze(
    Query(query): Query<SampleQuery>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut file_bytes = Vec::new();
    let mut filename = "uploaded.jpg".to_string();
    let sigma = query.sigma.unwrap_or(2.2);

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
        return Json(run_full_pipeline_with_sigma(&loaded, sigma));
    }

    let loaded = match load_image_from_bytes(&filename, &file_bytes) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to load uploaded image '{}': {}", filename, e);
            let opts = SyntheticOptions::default();
            generate_synthetic_image(&opts)
        }
    };

    Json(run_full_pipeline_with_sigma(&loaded, sigma))
}

pub fn encode_image_data_url(loaded: &crate::image_loader::LoadedImage) -> String {
    let mut buffer = Cursor::new(Vec::new());
    if loaded.rgb.write_to(&mut buffer, ImageFormat::Jpeg).is_ok() {
        let b64 = base64::engine::general_purpose::STANDARD.encode(buffer.get_ref());
        format!("data:image/jpeg;base64,{b64}")
    } else {
        String::new()
    }
}

pub fn run_full_pipeline(loaded: &crate::image_loader::LoadedImage) -> AnalysisPipelineResult {
    run_full_pipeline_with_sigma(loaded, 2.2)
}

pub fn run_full_pipeline_with_sigma(
    loaded: &crate::image_loader::LoadedImage,
    sigma: f64,
) -> AnalysisPipelineResult {
    let settings = DetectionSettings {
        sigma_threshold: sigma,
        ..Default::default()
    };

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
        Some(&loaded.rgb),
    );

    let streaks = detect_satellite_streaks(&loaded.gray, detection.horizon_y);
    let sat_matches = match_satellites_with_sgp4(&streaks, ts);

    let image_data_url = encode_image_data_url(loaded);

    AnalysisPipelineResult {
        image_name: loaded.name.clone(),
        image_data_url,
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
            --accent-red: #f87171;
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
            max-width: 1600px;
            margin: 1.5rem auto;
            padding: 0 1.5rem;
            display: grid;
            grid-template-columns: 2.1fr 1fr;
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
            margin-bottom: 0.8rem;
            color: var(--accent-purple);
            border-bottom: 1px solid var(--border-color);
            padding-bottom: 0.5rem;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }

        .controls-bar {
            display: flex;
            flex-wrap: wrap;
            align-items: center;
            justify-content: space-between;
            gap: 1rem;
            background: rgba(15, 23, 42, 0.6);
            padding: 0.8rem 1rem;
            border-radius: 8px;
            margin-bottom: 1rem;
            border: 1px solid var(--border-color);
            font-size: 0.85rem;
        }

        .controls-group {
            display: flex;
            flex-wrap: wrap;
            gap: 1rem;
            align-items: center;
        }

        .controls-bar label {
            display: flex;
            align-items: center;
            gap: 6px;
            cursor: pointer;
            user-select: none;
        }

        .canvas-container {
            position: relative;
            width: 100%;
            background: #000;
            border-radius: 8px;
            overflow: hidden;
            border: 1px solid var(--border-color);
            cursor: grab;
        }

        .canvas-container:active {
            cursor: grabbing;
        }

        canvas {
            display: block;
            width: 100%;
            height: auto;
        }

        .zoom-toolbar {
            position: absolute;
            top: 12px;
            right: 12px;
            display: flex;
            gap: 6px;
            background: rgba(15, 23, 42, 0.85);
            backdrop-filter: blur(8px);
            padding: 6px;
            border-radius: 8px;
            border: 1px solid var(--border-color);
            z-index: 10;
        }

        .zoom-toolbar button {
            padding: 0.3rem 0.6rem;
            font-size: 0.85rem;
        }

        .hud-overlay {
            position: absolute;
            bottom: 12px;
            left: 12px;
            background: rgba(15, 23, 42, 0.85);
            backdrop-filter: blur(8px);
            padding: 6px 12px;
            border-radius: 6px;
            border: 1px solid var(--border-color);
            font-size: 0.78rem;
            color: var(--accent-blue);
            pointer-events: none;
            z-index: 10;
        }

        /* Nav Tabs for Right Dashboard Panel */
        .tab-buttons {
            display: flex;
            gap: 4px;
            margin-bottom: 1rem;
            border-bottom: 1px solid var(--border-color);
            overflow-x: auto;
        }

        .tab-btn {
            background: rgba(255, 255, 255, 0.05);
            border: 1px solid transparent;
            color: var(--text-muted);
            padding: 0.5rem 0.8rem;
            border-radius: 6px 6px 0 0;
            font-weight: 600;
            font-size: 0.82rem;
            cursor: pointer;
            white-space: nowrap;
        }

        .tab-btn.active {
            background: var(--card-bg);
            color: var(--accent-blue);
            border-color: var(--border-color) var(--border-color) transparent var(--border-color);
        }

        .tab-content {
            display: none;
        }

        .tab-content.active {
            display: block;
        }

        .metrics-grid {
            display: grid;
            grid-template-columns: repeat(2, 1fr);
            gap: 0.8rem;
            margin-bottom: 1rem;
        }

        .metric-box {
            background: rgba(255, 255, 255, 0.03);
            border: 1px solid var(--border-color);
            border-radius: 8px;
            padding: 0.7rem;
        }

        .metric-title {
            font-size: 0.75rem;
            color: var(--text-muted);
            text-transform: uppercase;
        }

        .metric-value {
            font-size: 1.2rem;
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
            padding: 1.2rem;
            text-align: center;
            margin-bottom: 1rem;
            background: rgba(56, 189, 248, 0.05);
        }

        table.data-table {
            width: 100%;
            border-collapse: collapse;
            font-size: 0.82rem;
            margin-top: 0.5rem;
        }

        table.data-table th, table.data-table td {
            padding: 0.5rem;
            text-align: left;
            border-bottom: 1px solid rgba(255, 255, 255, 0.06);
        }

        table.data-table th {
            color: var(--accent-purple);
            font-weight: 600;
        }

        .progress-bar-bg {
            background: rgba(255, 255, 255, 0.1);
            border-radius: 6px;
            height: 10px;
            width: 100%;
            overflow: hidden;
            margin-top: 6px;
        }

        .progress-bar-fill {
            height: 100%;
            background: linear-gradient(90deg, #4ade80, #38bdf8);
            border-radius: 6px;
            transition: width 0.3s;
        }

        ul {
            list-style: none;
            padding: 0;
            margin: 0;
        }

        li {
            padding: 0.45rem 0;
            border-bottom: 1px solid rgba(255,255,255,0.05);
            font-size: 0.85rem;
        }

        .badge-err {
            display: inline-block;
            padding: 2px 6px;
            border-radius: 4px;
            font-size: 0.75rem;
            font-weight: bold;
        }
        .badge-green { background: rgba(74, 222, 128, 0.2); color: var(--accent-green); }
        .badge-yellow { background: rgba(250, 204, 21, 0.2); color: var(--accent-yellow); }
        .badge-red { background: rgba(248, 113, 113, 0.2); color: var(--accent-red); }

        input[type=range] {
            accent-color: var(--accent-blue);
            cursor: pointer;
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
            <h2>
                Interactive Celestial Map (Scroll Wheel to Zoom | Click & Drag to Pan)
                <span id="zoomDisplay" style="font-size:0.85rem; color: var(--accent-blue);">Zoom: 100%</span>
            </h2>

            <div class="controls-bar">
                <div class="controls-group">
                    <label><input type="checkbox" id="chkImage" checked onchange="redrawCanvas()"> 📷 Background</label>
                    <label><input type="checkbox" id="chkStars" checked onchange="redrawCanvas()"> ⭐ Stars</label>
                    <label><input type="checkbox" id="chkCatalog" checked onchange="redrawCanvas()"> 🎯 Catalog</label>
                    <label><input type="checkbox" id="chkErrorVectors" checked onchange="redrawCanvas()"> 📐 Errors</label>
                    <label><input type="checkbox" id="chkGrid" checked onchange="redrawCanvas()"> 🌐 Grid</label>
                    <label><input type="checkbox" id="chkSatellites" checked onchange="redrawCanvas()"> 🛰️ Satellites</label>
                </div>

                <div class="controls-group">
                    <label>Sensitivity Threshold: <span id="sigmaVal" style="font-weight:bold; color:var(--accent-yellow);">2.2 σ</span></label>
                    <input type="range" id="sigmaSlider" min="1.0" max="5.0" step="0.1" value="2.2" onchange="onSigmaChange(this.value)">
                </div>
            </div>

            <div class="canvas-container" id="canvasBox">
                <div class="zoom-toolbar">
                    <button onclick="zoomIn()">🔍 +</button>
                    <button onclick="zoomOut()">🔍 -</button>
                    <button onclick="resetZoom()">🎯 Reset View</button>
                </div>
                <div class="hud-overlay" id="hudText">Cursor Position: X: 0, Y: 0</div>
                <canvas id="starCanvas" width="1200" height="900"></canvas>
            </div>
        </div>

        <div style="display: flex; flex-direction: column; gap: 1.2rem;">
            <div class="card">
                <h2>Upload Image File</h2>
                <div class="upload-area">
                    <input type="file" id="fileInput" accept="image/*" style="display: none;">
                    <p style="margin-top:0;">Select iPhone Astrophotography Image (JPG, PNG, HEIC)</p>
                    <button onclick="document.getElementById('fileInput').click()">Browse File</button>
                </div>
            </div>

            <div class="card">
                <div class="tab-buttons">
                    <button class="tab-btn active" onclick="switchDashboardTab(0)">📊 Overview</button>
                    <button class="tab-btn" onclick="switchDashboardTab(1)">🎯 Catalog</button>
                    <button class="tab-btn" onclick="switchDashboardTab(2)">📷 EXIF Drift</button>
                    <button class="tab-btn" onclick="switchDashboardTab(3)">🔬 Aberration</button>
                    <button class="tab-btn" onclick="switchDashboardTab(4)">🛰️ Satellites</button>
                </div>

                <!-- Tab 0: Overview -->
                <div class="tab-content active" id="tabOverview">
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
                            <div class="metric-title">RMS Residual Error</div>
                            <div class="metric-value" id="valRmse">0.0 px</div>
                        </div>
                        <div class="metric-box">
                            <div class="metric-title">Optical Quality Index</div>
                            <div class="metric-value" id="valQuality">0 / 100</div>
                        </div>
                    </div>

                    <div style="margin-top: 0.8rem;">
                        <label style="font-size: 0.8rem; color: var(--text-muted);">Optical Health Score Gauge:</label>
                        <div class="progress-bar-bg">
                            <div class="progress-bar-fill" id="qualityGaugeBar" style="width: 0%;"></div>
                        </div>
                    </div>

                    <ul id="overviewList" style="margin-top: 0.8rem;">
                        <li>Loading execution details...</li>
                    </ul>
                </div>

                <!-- Tab 1: Solved Catalog Table -->
                <div class="tab-content" id="tabCatalog">
                    <div style="max-height: 380px; overflow-y: auto;">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>Catalog Star</th>
                                    <th>RA (°)</th>
                                    <th>Dec (°)</th>
                                    <th>Mag</th>
                                    <th>Residual</th>
                                </tr>
                            </thead>
                            <tbody id="catalogTableBody">
                                <tr><td colspan="5">No catalog data available</td></tr>
                            </tbody>
                        </table>
                    </div>
                </div>

                <!-- Tab 2: EXIF & Astrometric Drift -->
                <div class="tab-content" id="tabExif">
                    <ul id="exifList">
                        <li>No EXIF metadata parsed</li>
                    </ul>
                </div>

                <!-- Tab 3: Aberration & Refraction -->
                <div class="tab-content" id="tabAberration">
                    <ul id="aberrationList">
                        <li>No aberration data available</li>
                    </ul>
                </div>

                <!-- Tab 4: Satellites & SGP4 Orbit -->
                <div class="tab-content" id="tabSatellites">
                    <ul id="satellitesList">
                        <li>No satellite tracking data</li>
                    </ul>
                </div>
            </div>
        </div>
    </div>

    <script>
        let currentData = null;
        let currentImg = null;
        let currentSource = 'sample';
        let currentFile = null;

        // Zoom & Pan State
        let scale = 1.0;
        let panX = 0;
        let panY = 0;
        let isDragging = false;
        let startX = 0;
        let startY = 0;

        function switchDashboardTab(idx) {
            const btns = document.querySelectorAll('.tab-btn');
            const contents = document.querySelectorAll('.tab-content');

            btns.forEach((btn, i) => {
                if (i === idx) btn.classList.add('active');
                else btn.classList.remove('active');
            });

            contents.forEach((cnt, i) => {
                if (i === idx) cnt.classList.add('active');
                else cnt.classList.remove('active');
            });
        }

        async function loadSampleData() {
            currentSource = 'sample';
            const sigma = document.getElementById('sigmaSlider').value;
            const res = await fetch(`/api/sample?sigma=${sigma}`);
            currentData = await res.json();
            processLoadedData();
        }

        function onSigmaChange(val) {
            document.getElementById('sigmaVal').innerText = parseFloat(val).toFixed(1) + ' σ';
            if (currentSource === 'file' && currentFile) {
                uploadSelectedFile(currentFile);
            } else {
                loadSampleData();
            }
        }

        document.getElementById('fileInput').addEventListener('change', async (e) => {
            const file = e.target.files[0];
            if (!file) return;

            currentFile = file;
            currentSource = 'file';
            uploadSelectedFile(file);
        });

        async function uploadSelectedFile(file) {
            const sigma = document.getElementById('sigmaSlider').value;
            const formData = new FormData();
            formData.append('file', file);

            const res = await fetch(`/api/upload?sigma=${sigma}`, {
                method: 'POST',
                body: formData
            });

            currentData = await res.json();
            processLoadedData();
        }

        function processLoadedData() {
            if (!currentData) return;

            const starCount = currentData.detection.stars.length;
            const rmse = currentData.solution.rmse_pixels;
            const quality = currentData.aberration.quality_score;

            document.getElementById('valStars').innerText = starCount;
            document.getElementById('valSolved').innerText = currentData.solution.solved ? "SOLVED" : "UNSOLVED";
            document.getElementById('valRmse').innerText = rmse.toFixed(2) + " px";
            document.getElementById('valQuality').innerText = quality.toFixed(1) + " / 100";

            document.getElementById('qualityGaugeBar').style.width = Math.min(100, Math.max(0, quality)).toFixed(0) + '%';

            const badgeClass = rmse < 3.0 ? 'badge-green' : (rmse < 8.0 ? 'badge-yellow' : 'badge-red');

            // 1. Overview List
            document.getElementById('overviewList').innerHTML = `
                <li><b>Image Name:</b> ${currentData.image_name} (${currentData.width}x${currentData.height} px)</li>
                <li><b>Camera Model:</b> ${currentData.exif.make || 'Apple'} ${currentData.exif.model || 'iPhone'}</li>
                <li><b>Horizon Boundary:</b> ${currentData.detection.horizon_y ? 'y = ' + currentData.detection.horizon_y + ' px' : 'None'}</li>
                <li><b>Catalog Matches:</b> ${currentData.solution.matches.length} stars</li>
                <li><b>Residual Status:</b> <span class="badge-err ${badgeClass}">${rmse.toFixed(2)} px</span></li>
            `;

            // 2. Catalog Table
            const catBody = document.getElementById('catalogTableBody');
            if (currentData.solution.matches.length > 0) {
                catBody.innerHTML = currentData.solution.matches.map(m => {
                    const err = m.residual_pixels;
                    const badge = err < 3.0 ? 'badge-green' : (err < 8.0 ? 'badge-yellow' : 'badge-red');
                    return `
                        <tr>
                            <td><b>${m.catalog_name}</b></td>
                            <td>${m.catalog_ra.toFixed(2)}°</td>
                            <td>${m.catalog_dec.toFixed(2)}°</td>
                            <td>${m.catalog_vmag.toFixed(2)}</td>
                            <td><span class="badge-err ${badge}">${err.toFixed(2)} px</span></td>
                        </tr>
                    `;
                }).join('');
            } else {
                catBody.innerHTML = '<tr><td colspan="5">No matched catalog stars</td></tr>';
            }

            // 3. EXIF & Validation List
            document.getElementById('exifList').innerHTML = `
                <li><b>Latitude:</b> ${currentData.exif.latitude ? currentData.exif.latitude.toFixed(6) + '°' : 'N/A'}</li>
                <li><b>Longitude:</b> ${currentData.exif.longitude ? currentData.exif.longitude.toFixed(6) + '°' : 'N/A'}</li>
                <li><b>Altitude:</b> ${currentData.exif.altitude ? currentData.exif.altitude.toFixed(1) + ' m' : 'N/A'}</li>
                <li><b>Compass Heading:</b> ${currentData.exif.heading_deg ? currentData.exif.heading_deg.toFixed(1) + '°' : '180.0°'}</li>
                <li><b>Focal Length:</b> ${currentData.exif.focal_length_in_35mm || 26} mm equiv</li>
                <li><b>ISO & Exposure:</b> ISO ${currentData.exif.iso || 1600}, t = ${currentData.exif.exposure_time ? currentData.exif.exposure_time.toFixed(3) + 's' : 'N/A'}</li>
                <li><b>Time Drift Offset:</b> ${currentData.validation.time_drift_seconds.toFixed(1)} s</li>
                <li><b>Heading Adjustment:</b> ${currentData.validation.heading_error_deg.toFixed(2)}°</li>
                <li><b>Validation Status:</b> ${currentData.validation.summary}</li>
            `;

            // 4. Aberration & Refraction List
            let sipHtml = '<li><b>SIP Model:</b> Linear (N=1)</li>';
            if (currentData.solution.sip_distortion) {
                const sip = currentData.solution.sip_distortion;
                let coeffStr = '';
                for (let p = 0; p <= sip.order; p++) {
                    for (let q = 0; q <= sip.order; q++) {
                        if (p + q >= 2 && p + q <= sip.order) {
                            if (sip.a[p][q] !== 0) coeffStr += `A<sub>${p},${q}</sub>=${sip.a[p][q].toExponential(2)} `;
                            if (sip.b[p][q] !== 0) coeffStr += `B<sub>${p},${q}</sub>=${sip.b[p][q].toExponential(2)} `;
                        }
                    }
                }
                const sipErrBadge = sip.fit_rmse_pixels < 2.0 ? 'badge-green' : (sip.fit_rmse_pixels < 5.0 ? 'badge-yellow' : 'badge-red');
                sipHtml = `
                    <li><b>SIP Distortion Model:</b> Polynomial Order N = ${sip.order}</li>
                    <li><b>SIP Fit Error (RMSE):</b> <span class="badge-err ${sipErrBadge}">${sip.fit_rmse_pixels.toFixed(3)} px</span></li>
                    <li><b>SIP Coefficients:</b><br><small style="color:var(--accent-yellow);">${coeffStr || 'Negligible'}</small></li>
                `;
            }

            const radialErrBadge = currentData.aberration.radial_fit_rmse_px < 2.0 ? 'badge-green' : (currentData.aberration.radial_fit_rmse_px < 5.0 ? 'badge-yellow' : 'badge-red');

            document.getElementById('aberrationList').innerHTML = `
                ${sipHtml}
                <li><b>Plate Solve Fit Error (RMSE):</b> <span class="badge-err ${badgeClass}">${rmse.toFixed(2)} px</span></li>
                <li><b>Radial Fit Error (RMSE):</b> <span class="badge-err ${radialErrBadge}">${currentData.aberration.radial_fit_rmse_px.toFixed(3)} px</span></li>
                <li><b>Radial Distortion (k1):</b> ${currentData.aberration.radial_k1.toFixed(6)}</li>
                <li><b>Radial Distortion (k2):</b> ${currentData.aberration.radial_k2.toFixed(6)}</li>
                <li><b>Coma Factor:</b> ${currentData.aberration.coma_factor.toFixed(4)}</li>
                <li><b>Astigmatism Factor:</b> ${currentData.aberration.astigmatism_factor.toFixed(4)}</li>
                <li><b>Chromatic Aberration:</b> ${currentData.aberration.chromatic_aberration_px.toFixed(2)} px</li>
                <li><b>Atmospheric Refraction Correction:</b> ${currentData.aberration.atmospheric_refraction_arcmin.toFixed(2)} arcmin</li>
                <li><b>Max Radial Shift:</b> ${currentData.aberration.max_radial_distortion_px.toFixed(1)} px</li>
                <li><b>Quality Health Score:</b> ${currentData.aberration.quality_score.toFixed(1)} / 100</li>
            `;

            // 5. Satellites & SGP4 List
            const satList = document.getElementById('satellitesList');
            let satHtml = '';
            if (currentData.satellite_report.streaks.length > 0) {
                satHtml += currentData.satellite_report.streaks.map(s =>
                    `<li><b>Streak #${s.id}:</b> Length = ${s.length_px.toFixed(1)} px, Angle = ${s.angle_deg.toFixed(1)}°, Brightness = ${s.brightness.toFixed(1)}</li>`
                ).join('');
            } else {
                satHtml += '<li>No linear satellite streaks detected</li>';
            }

            if (currentData.satellite_report.matches.length > 0) {
                satHtml += currentData.satellite_report.matches.map(m =>
                    `<li><b>Matched Orbit:</b> ${m.name} (NORAD ${m.norad_id}) — Conf: ${(m.confidence * 100).toFixed(0)}%<br>` +
                    `<small style="color:var(--text-muted);">Position: X=${m.position_km[0].toFixed(1)} km, Y=${m.position_km[1].toFixed(1)} km, Z=${m.position_km[2].toFixed(1)} km</small></li>`
                ).join('');
            }
            satList.innerHTML = satHtml;

            if (currentData.image_data_url) {
                currentImg = new Image();
                currentImg.onload = () => redrawCanvas();
                currentImg.src = currentData.image_data_url;
            } else {
                currentImg = null;
                redrawCanvas();
            }
        }

        // --- Canvas Zoom & Pan Handlers ---
        const canvasBox = document.getElementById('canvasBox');
        const canvas = document.getElementById('starCanvas');

        canvasBox.addEventListener('wheel', (e) => {
            e.preventDefault();
            const zoomFactor = e.deltaY < 0 ? 1.15 : 0.85;

            const rect = canvas.getBoundingClientRect();
            const mouseX = e.clientX - rect.left;
            const mouseY = e.clientY - rect.top;

            const newScale = Math.min(Math.max(0.5, scale * zoomFactor), 15.0);

            panX = mouseX - (mouseX - panX) * (newScale / scale);
            panY = mouseY - (mouseY - panY) * (newScale / scale);

            scale = newScale;
            updateZoomDisplay();
            redrawCanvas();
        }, { passive: false });

        canvasBox.addEventListener('mousedown', (e) => {
            isDragging = true;
            startX = e.clientX - panX;
            startY = e.clientY - panY;
        });

        window.addEventListener('mousemove', (e) => {
            const rect = canvas.getBoundingClientRect();
            if (e.clientX >= rect.left && e.clientX <= rect.right && e.clientY >= rect.top && e.clientY <= rect.bottom) {
                const imgX = ((e.clientX - rect.left - panX) / (scale * (rect.width / canvas.width))).toFixed(1);
                const imgY = ((e.clientY - rect.top - panY) / (scale * (rect.height / canvas.height))).toFixed(1);
                document.getElementById('hudText').innerText = `Cursor Position: X: ${imgX}, Y: ${imgY}`;
            }

            if (!isDragging) return;
            panX = e.clientX - startX;
            panY = e.clientY - startY;
            redrawCanvas();
        });

        window.addEventListener('mouseup', () => { isDragging = false; });

        function zoomIn() {
            scale = Math.min(15.0, scale * 1.3);
            updateZoomDisplay();
            redrawCanvas();
        }

        function zoomOut() {
            scale = Math.max(0.5, scale / 1.3);
            updateZoomDisplay();
            redrawCanvas();
        }

        function resetZoom() {
            scale = 1.0;
            panX = 0;
            panY = 0;
            updateZoomDisplay();
            redrawCanvas();
        }

        function updateZoomDisplay() {
            document.getElementById('zoomDisplay').innerText = `Zoom: ${(scale * 100).toFixed(0)}%`;
        }

        function redrawCanvas() {
            if (!currentData) return;

            const ctx = canvas.getContext('2d');
            canvas.width = currentData.width;
            canvas.height = currentData.height;

            ctx.clearRect(0, 0, canvas.width, canvas.height);
            ctx.save();

            ctx.translate(panX, panY);
            ctx.scale(scale, scale);

            const showImg = document.getElementById('chkImage').checked;
            const showStars = document.getElementById('chkStars').checked;
            const showCatalog = document.getElementById('chkCatalog').checked;
            const showErrorVectors = document.getElementById('chkErrorVectors').checked;
            const showGrid = document.getElementById('chkGrid').checked;
            const showSatellites = document.getElementById('chkSatellites').checked;

            if (showImg && currentImg && currentImg.complete) {
                ctx.drawImage(currentImg, 0, 0, canvas.width, canvas.height);
                ctx.fillStyle = 'rgba(11, 15, 25, 0.12)';
                ctx.fillRect(0, 0, canvas.width, canvas.height);
            } else {
                ctx.fillStyle = '#05070e';
                ctx.fillRect(0, 0, canvas.width, canvas.height);
            }

            if (showGrid) {
                const cx = canvas.width / 2;
                const cy = canvas.height / 2;
                const maxR = Math.hypot(cx, cy);
                const k1 = currentData.aberration.radial_k1;

                ctx.strokeStyle = 'rgba(192, 132, 252, 0.25)';
                ctx.lineWidth = 1 / scale;
                ctx.setLineDash([4 / scale, 4 / scale]);

                for (let r = 100; r < maxR; r += 150) {
                    ctx.beginPath();
                    ctx.arc(cx, cy, r, 0, 2 * Math.PI);
                    ctx.stroke();
                }

                for (let y = 100; y < canvas.height; y += 200) {
                    for (let x = 100; x < canvas.width; x += 250) {
                        const dx = x - cx;
                        const dy = y - cy;
                        const r2 = (dx*dx + dy*dy) / (maxR*maxR);
                        const distShift = k1 * r2 * 30.0;

                        ctx.strokeStyle = 'rgba(192, 132, 252, 0.4)';
                        ctx.beginPath();
                        ctx.moveTo(x, y);
                        ctx.lineTo(x + dx * distShift, y + dy * distShift);
                        ctx.stroke();
                    }
                }
                ctx.setLineDash([]);
            }

            if (showSatellites && currentData.detection.horizon_y) {
                const hy = currentData.detection.horizon_y;
                ctx.fillStyle = 'rgba(239, 68, 68, 0.15)';
                ctx.fillRect(0, hy, canvas.width, canvas.height - hy);

                ctx.strokeStyle = '#ef4444';
                ctx.lineWidth = 1.5 / scale;
                ctx.setLineDash([6 / scale, 6 / scale]);
                ctx.beginPath();
                ctx.moveTo(0, hy);
                ctx.lineTo(canvas.width, hy);
                ctx.stroke();
                ctx.setLineDash([]);
            }

            if (showStars) {
                currentData.detection.stars.forEach(star => {
                    ctx.fillStyle = '#facc15';
                    ctx.beginPath();
                    ctx.arc(star.x, star.y, Math.max(2.5 / scale, (star.fwhm * 0.8) / scale), 0, 2 * Math.PI);
                    ctx.fill();

                    ctx.strokeStyle = 'rgba(250, 204, 21, 0.5)';
                    ctx.lineWidth = 1.5 / scale;
                    ctx.beginPath();
                    ctx.arc(star.x, star.y, (star.fwhm * 2.2) / scale, 0, 2 * Math.PI);
                    ctx.stroke();

                    ctx.strokeStyle = 'rgba(255, 255, 255, 0.7)';
                    ctx.lineWidth = 1 / scale;
                    ctx.beginPath();
                    ctx.moveTo(star.x - 4 / scale, star.y); ctx.lineTo(star.x + 4 / scale, star.y);
                    ctx.moveTo(star.x, star.y - 4 / scale); ctx.lineTo(star.x, star.y + 4 / scale);
                    ctx.stroke();
                });
            }

            if (showCatalog) {
                currentData.solution.matches.forEach(m => {
                    const err = m.residual_pixels;
                    let color = '#4ade80';
                    if (err >= 8.0) color = '#f87171';
                    else if (err >= 3.0) color = '#facc15';

                    ctx.strokeStyle = color;
                    ctx.lineWidth = 2 / scale;
                    ctx.beginPath();
                    ctx.arc(m.pixel_x, m.pixel_y, 14 / scale, 0, 2 * Math.PI);
                    ctx.stroke();

                    ctx.beginPath();
                    ctx.moveTo(m.pixel_x - 18 / scale, m.pixel_y); ctx.lineTo(m.pixel_x - 10 / scale, m.pixel_y);
                    ctx.moveTo(m.pixel_x + 10 / scale, m.pixel_y); ctx.lineTo(m.pixel_x + 18 / scale, m.pixel_y);
                    ctx.moveTo(m.pixel_x, m.pixel_y - 18 / scale); ctx.lineTo(m.pixel_x, m.pixel_y - 10 / scale);
                    ctx.moveTo(m.pixel_x, m.pixel_y + 10 / scale); ctx.lineTo(m.pixel_x, m.pixel_y + 18 / scale);
                    ctx.stroke();

                    if (showErrorVectors) {
                        const detStar = currentData.detection.stars.find(s => s.id === m.star_id);
                        if (detStar) {
                            ctx.strokeStyle = color;
                            ctx.lineWidth = 1.5 / scale;
                            ctx.setLineDash([3 / scale, 3 / scale]);
                            ctx.beginPath();
                            ctx.moveTo(detStar.x, detStar.y);
                            ctx.lineTo(m.pixel_x, m.pixel_y);
                            ctx.stroke();
                            ctx.setLineDash([]);
                        }
                    }

                    ctx.fillStyle = '#ffffff';
                    ctx.font = `bold ${Math.max(10, 12 / scale)}px sans-serif`;
                    ctx.fillText(m.catalog_name, m.pixel_x + 22 / scale, m.pixel_y + 2 / scale);

                    ctx.fillStyle = color;
                    ctx.font = `${Math.max(9, 11 / scale)}px monospace`;
                    ctx.fillText(`err: ${err.toFixed(1)}px`, m.pixel_x + 22 / scale, m.pixel_y + 16 / scale);
                });
            }

            if (showSatellites) {
                currentData.satellite_report.streaks.forEach(s => {
                    ctx.strokeStyle = '#38bdf8';
                    ctx.lineWidth = 3 / scale;
                    ctx.beginPath();
                    ctx.moveTo(s.start_x, s.start_y);
                    ctx.lineTo(s.end_x, s.end_y);
                    ctx.stroke();

                    ctx.fillStyle = '#38bdf8';
                    ctx.font = `bold ${Math.max(10, 12 / scale)}px sans-serif`;
                    ctx.fillText(`Satellite Track #${s.id}`, s.start_x + 10 / scale, s.start_y - 10 / scale);
                });
            }

            ctx.restore();
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
        assert!(!result.image_data_url.is_empty());
    }
}
