mod app;
mod gcp;
mod ui;
mod types;

use anyhow::Result;
use clap::{Arg, Command};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io;

use app::App;
use ui::run_app;

#[tokio::main]
async fn main() -> Result<()> {
    let matches = Command::new("GCP SQL Backup Restore")
        .version("2.0.0")
        .about("Interactive GCP SQL Instance Backup Restore Tool")
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .help("Run in dry-run mode (simulate operations without executing)")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    let dry_run_mode = matches.get_flag("dry-run");

    // Run the application in restore mode (with or without dry-run)
    run_tui_app(dry_run_mode).await?;

    Ok(())
}

async fn run_tui_app(dry_run_mode: bool) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let app = App::new(dry_run_mode).await?;
    let res = run_app(&mut terminal, app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}
