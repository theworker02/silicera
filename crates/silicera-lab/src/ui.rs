//! Terminal lab UI — systems aesthetic, real measurements only.

use std::io::{self, Write};
use std::path::Path;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};
use ratatui::Terminal;
use serde::Serialize;
use silicera::brand::{BrandInfo, BRAND_LINE, NAME, RELEASE_LINE, VERSION};
use silicera::hardware::HardwareInfo;
use silicera::measure::{MeasurementConfig, MeasurementEngine, MeasurementSummary};

use crate::bench::{CacheTarget, IntegerBench, MemoryBench};

/// Row collected from a live measurement.
#[derive(Clone, Serialize)]
struct LiveRow {
    name: String,
    median_ns: f64,
    cv: f64,
    stability: String,
}

/// Exportable lab session summary (measured rows only).
#[derive(Debug, Clone, Serialize)]
pub struct LabSessionSummary {
    /// Product name.
    pub product: String,
    /// Silicera version.
    pub silicera_version: String,
    /// Phase marker.
    pub phase: String,
    /// Brand line.
    pub brand_line: String,
    /// Host CPU brand string.
    pub host_brand: String,
    /// Fingerprint when supported.
    pub fingerprint: Option<String>,
    /// RFC3339 capture time.
    pub captured_at: String,
    /// Measured workloads from the last pass.
    pub rows: Vec<LabSessionRow>,
    /// Caveats.
    pub caveats: Vec<String>,
}

/// One measured row in a lab session export.
#[derive(Debug, Clone, Serialize)]
pub struct LabSessionRow {
    /// Workload name.
    pub name: String,
    /// Median nanoseconds.
    pub median_ns: f64,
    /// Coefficient of variation.
    pub cv: f64,
    /// Stability label.
    pub stability: String,
}

impl LabSessionSummary {
    /// Build from host + measured session rows.
    pub fn from_rows(info: &HardwareInfo, rows: Vec<LabSessionRow>) -> Self {
        let brand = BrandInfo::current();
        Self {
            product: brand.name.into(),
            silicera_version: brand.version.into(),
            phase: brand.phase.into(),
            brand_line: brand.brand_line.into(),
            host_brand: info.brand.clone(),
            fingerprint: info.fingerprint.as_ref().map(|f| f.value.clone()),
            captured_at: chrono::Utc::now().to_rfc3339(),
            rows,
            caveats: vec![
                "Lab session exports contain real medians only — never fabricated speedups.".into(),
                format!("{NAME}; not affiliated with AMD."),
            ],
        }
    }

    /// Pretty JSON.
    pub fn to_json_pretty(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Write to path.
    pub fn write_to(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, self.to_json_pretty()?)?;
        Ok(())
    }
}

/// Coefficient of variation from a measurement summary (shared helper).
pub fn summary_cv(s: &MeasurementSummary) -> f64 {
    if s.mean_ns > 0.0 {
        s.stddev_ns / s.mean_ns
    } else {
        0.0
    }
}

/// Run the interactive lab UI. Returns when the user quits.
pub fn run_lab_ui(info: &HardwareInfo) -> anyhow::Result<()> {
    let mut stdout = io::stdout();
    enable_raw_mode()?;
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut rows: Vec<LiveRow> = Vec::new();
    let mut status =
        "press R to measure · E to export session JSON · Q to quit".to_string();
    let mut measuring = false;

    let result = loop {
        terminal.draw(|f| draw(f, info, &rows, &status, measuring))?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break Ok(()),
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        break Ok(())
                    }
                    KeyCode::Char('e') | KeyCode::Char('E') if !measuring => {
                        if rows.is_empty() {
                            status = "nothing to export — run a measurement pass first (R)".into();
                        } else {
                            let path = Path::new("out/lab-session.json");
                            let export_rows: Vec<LabSessionRow> = rows
                                .iter()
                                .map(|r| LabSessionRow {
                                    name: r.name.clone(),
                                    median_ns: r.median_ns,
                                    cv: r.cv,
                                    stability: r.stability.clone(),
                                })
                                .collect();
                            match LabSessionSummary::from_rows(info, export_rows).write_to(path) {
                                Ok(()) => {
                                    status = format!(
                                        "exported {} rows → {}",
                                        rows.len(),
                                        path.display()
                                    );
                                }
                                Err(e) => status = format!("export error: {e}"),
                            }
                        }
                    }
                    KeyCode::Char('r') | KeyCode::Char('R') if !measuring => {
                        measuring = true;
                        status = "measuring…".into();
                        terminal.draw(|f| draw(f, info, &rows, &status, measuring))?;
                        match run_pass(info) {
                            Ok(new_rows) => {
                                rows = new_rows;
                                status = format!(
                                    "pass complete — {} workloads (real medians only) · E exports JSON",
                                    rows.len()
                                );
                            }
                            Err(e) => status = format!("measurement error: {e}"),
                        }
                        measuring = false;
                    }
                    _ => {}
                }
            }
        }
    };

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    let _ = io::stdout().flush();
    result
}

fn run_pass(info: &HardwareInfo) -> anyhow::Result<Vec<LiveRow>> {
    let eng = MeasurementEngine::new(MeasurementConfig {
        warmup: 2,
        iterations: 12,
        ..Default::default()
    });
    let mut out = Vec::new();

    for target in [CacheTarget::L1, CacheTarget::L2, CacheTarget::L3, CacheTarget::Dram] {
        let bench = MemoryBench::for_target(&info.topology, target);
        let s = eng.measure(|| {
            let _ = bench.run();
        })?;
        out.push(LiveRow {
            name: format!("memory {}", target.describe(&info.topology)),
            median_ns: s.median_ns,
            cv: summary_cv(&s),
            stability: s.stability.label().into(),
        });
    }

    let ib = IntegerBench { n: 99 };
    let s = eng.measure(|| {
        let _ = ib.run_baseline();
    })?;
    out.push(LiveRow {
        name: "integer lcg".into(),
        median_ns: s.median_ns,
        cv: summary_cv(&s),
        stability: s.stability.label().into(),
    });

    Ok(out)
}

fn draw(
    f: &mut ratatui::Frame,
    info: &HardwareInfo,
    rows: &[LiveRow],
    status: &str,
    measuring: bool,
) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(4),
            Constraint::Min(8),
            Constraint::Length(2),
        ])
        .split(area);

    let title = Paragraph::new(vec![
        Line::from(Span::styled(
            BRAND_LINE,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!("lab — {RELEASE_LINE} · v{VERSION} — live measurements"),
            Style::default().fg(Color::Gray),
        )),
    ])
    .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(title, chunks[0]);

    let fp = info
        .fingerprint
        .as_ref()
        .map(|f| f.value.as_str())
        .unwrap_or("(unsupported)");
    let host = Paragraph::new(vec![
        Line::from(format!("host  {}", info.brand)),
        Line::from(format!("fp    {fp}")),
        Line::from(format!(
            "topo  cores={} threads={} domains={}",
            info.topology.core_count(),
            info.topology.thread_count(),
            info.topology.domain_count()
        )),
    ]);
    f.render_widget(host, chunks[1]);

    let header = Row::new(vec!["workload", "median_ns", "cv", "stability"]).style(
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );
    let table_rows: Vec<Row> = rows
        .iter()
        .map(|r| {
            Row::new(vec![
                r.name.clone(),
                format!("{:.0}", r.median_ns),
                format!("{:.3}", r.cv),
                r.stability.clone(),
            ])
        })
        .collect();
    let table = Table::new(
        table_rows,
        [
            Constraint::Percentage(55),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title("results"));
    f.render_widget(table, chunks[2]);

    let tip = if measuring { "…" } else { status };
    let footer = Paragraph::new(tip).style(Style::default().fg(Color::DarkGray));
    f.render_widget(footer, chunks[3]);

    let _ = Rect::default();
}
