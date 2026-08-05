use crate::aberration::AberrationReport;
use crate::astrometry::AstrometricSolution;
use crate::exif::ExifMetadata;
use crate::satellites::SatelliteReport;
use crate::star_finder::DetectionResult;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        canvas::{Canvas, Line as CanvasLine, Points},
        Block, Borders, Gauge, List, ListItem, Paragraph, Tabs,
    },
    Terminal,
};
use std::io::stdout;

pub struct TuiAppState {
    pub active_tab: usize,
    pub image_name: String,
    pub image_width: u32,
    pub image_height: u32,
    pub exif: ExifMetadata,
    pub detection: DetectionResult,
    pub solution: AstrometricSolution,
    pub aberration: AberrationReport,
    pub satellite_report: SatelliteReport,
}

pub fn run_tui(state: TuiAppState) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut current_tab = 0;

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(3)])
                .split(f.area());

            // Header Tabs
            let titles = [" [1] Overview ", " [2] Star Canvas ", " [3] EXIF & Plate Solve ", " [4] Aberration ", " [5] Satellites "];
            let tabs = Tabs::new(titles.iter().cloned().map(Line::from).collect::<Vec<_>>())
                .block(Block::default().borders(Borders::ALL).title(" ✦ iPhone Star Recognition & Aberration Analyzer (Ratatui TUI) ✦ "))
                .select(current_tab)
                .style(Style::default().fg(Color::Cyan))
                .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
            f.render_widget(tabs, chunks[0]);

            // Main Content Area based on Active Tab
            match current_tab {
                0 => render_overview_tab(f, chunks[1], &state),
                1 => render_canvas_tab(f, chunks[1], &state),
                2 => render_exif_tab(f, chunks[1], &state),
                3 => render_aberration_tab(f, chunks[1], &state),
                4 => render_satellites_tab(f, chunks[1], &state),
                _ => {}
            }

            // Footer / Navigation Guide
            let footer = Paragraph::new(" Navigation: [1-5] Switch Tabs | [Q / Esc] Quit | Wol Pumba Astrophotography Tool ")
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(footer, chunks[2]);
        })?;

        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('1') => current_tab = 0,
                    KeyCode::Char('2') => current_tab = 1,
                    KeyCode::Char('3') => current_tab = 2,
                    KeyCode::Char('4') => current_tab = 3,
                    KeyCode::Char('5') => current_tab = 4,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn render_overview_tab(f: &mut ratatui::Frame, area: Rect, state: &TuiAppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let info_text = vec![
        Line::from(vec![
            Span::styled("Image File: ", Style::default().fg(Color::Cyan)),
            Span::raw(&state.image_name),
        ]),
        Line::from(vec![
            Span::styled("Resolution: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{}x{} px", state.image_width, state.image_height)),
        ]),
        Line::from(vec![
            Span::styled("Camera Model: ", Style::default().fg(Color::Cyan)),
            Span::raw(state.exif.model.as_deref().unwrap_or("Unknown")),
        ]),
        Line::from(vec![
            Span::styled("Detected Stars: ", Style::default().fg(Color::Green)),
            Span::raw(format!("{}", state.detection.stars.len())),
        ]),
        Line::from(vec![
            Span::styled("Plate Solved: ", Style::default().fg(Color::Yellow)),
            Span::raw(if state.solution.solved {
                "YES (SUCCESS)"
            } else {
                "NO"
            }),
        ]),
        Line::from(vec![
            Span::styled("Matched Catalog Stars: ", Style::default().fg(Color::Green)),
            Span::raw(format!("{}", state.solution.matches.len())),
        ]),
        Line::from(vec![
            Span::styled(
                "Optical Quality Score: ",
                Style::default().fg(Color::Magenta),
            ),
            Span::raw(format!("{:.1} / 100", state.aberration.quality_score)),
        ]),
    ];

    let p1 = Paragraph::new(info_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Execution Metrics "),
    );
    f.render_widget(p1, chunks[0]);

    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Optical Health Index "),
        )
        .gauge_style(Style::default().fg(Color::Green))
        .percent(state.aberration.quality_score.round() as u16);
    f.render_widget(gauge, chunks[1]);
}

fn render_canvas_tab(f: &mut ratatui::Frame, area: Rect, state: &TuiAppState) {
    let canvas = Canvas::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Celestial Star Map Overlay (Pixel Grid) "),
        )
        .x_bounds([0.0, state.image_width as f64])
        .y_bounds([0.0, state.image_height as f64])
        .paint(|ctx| {
            // Draw Ground Horizon Line
            if let Some(hy) = state.detection.horizon_y {
                ctx.draw(&CanvasLine {
                    x1: 0.0,
                    y1: hy as f64,
                    x2: state.image_width as f64,
                    y2: hy as f64,
                    color: Color::Red,
                });
            }

            // Draw Stars as Points
            let star_points: Vec<(f64, f64)> =
                state.detection.stars.iter().map(|s| (s.x, s.y)).collect();
            ctx.draw(&Points {
                coords: &star_points,
                color: Color::Yellow,
            });

            // Draw Satellite Streaks
            for streak in &state.satellite_report.streaks {
                ctx.draw(&CanvasLine {
                    x1: streak.start_x,
                    y1: streak.start_y,
                    x2: streak.end_x,
                    y2: streak.end_y,
                    color: Color::Cyan,
                });
            }
        });

    f.render_widget(canvas, area);
}

fn render_exif_tab(f: &mut ratatui::Frame, area: Rect, state: &TuiAppState) {
    let items = vec![
        ListItem::new(format!(
            "Latitude: {:?}",
            state.exif.latitude.unwrap_or(0.0)
        )),
        ListItem::new(format!(
            "Longitude: {:?}",
            state.exif.longitude.unwrap_or(0.0)
        )),
        ListItem::new(format!(
            "Heading: {:?}°",
            state.exif.heading_deg.unwrap_or(0.0)
        )),
        ListItem::new(format!(
            "Focal Length: {:?}mm",
            state.exif.focal_length_in_35mm.unwrap_or(26.0)
        )),
        ListItem::new(format!("RMS Error: {:.2} px", state.solution.rmse_pixels)),
    ];
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" EXIF Metadata & Solved Catalog "),
    );
    f.render_widget(list, area);
}

fn render_aberration_tab(f: &mut ratatui::Frame, area: Rect, state: &TuiAppState) {
    let text = vec![
        Line::from(format!(
            "Radial Distortion Polynomial (k1): {:.6}",
            state.aberration.radial_k1
        )),
        Line::from(format!(
            "Radial Distortion Polynomial (k2): {:.6}",
            state.aberration.radial_k2
        )),
        Line::from(format!("Coma Factor: {:.4}", state.aberration.coma_factor)),
        Line::from(format!(
            "Astigmatism Factor: {:.4}",
            state.aberration.astigmatism_factor
        )),
        Line::from(format!(
            "Atmospheric Refraction: {:.2} arcmin",
            state.aberration.atmospheric_refraction_arcmin
        )),
    ];
    let p = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Camera Aberration Analysis "),
    );
    f.render_widget(p, area);
}

fn render_satellites_tab(f: &mut ratatui::Frame, area: Rect, state: &TuiAppState) {
    let mut items = Vec::new();
    for streak in &state.satellite_report.streaks {
        items.push(ListItem::new(format!(
            "Streak #{}: Length {:.1}px, Angle {:.1}°",
            streak.id, streak.length_px, streak.angle_deg
        )));
    }
    for m in &state.satellite_report.matches {
        items.push(ListItem::new(format!(
            "Matched Satellite: {} (NORAD {})",
            m.name, m.norad_id
        )));
    }
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Satellite Trackers & SGP4 "),
    );
    f.render_widget(list, area);
}
