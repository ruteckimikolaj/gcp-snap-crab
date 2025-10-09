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
use types::OperationMode;

#[tokio::main]
async fn main() -> Result<()> {
    let matches = Command::new("GCP SQL Backup Tool")
        .version("2.0.0")
        .about("Interactive GCP SQL Instance Backup and Restore Tool")
        .arg(
            Arg::new("operation")
                .long("operation")
                .value_name("MODE")
                .help("The operation to perform")
                .value_parser(["restore", "create-backup"])
                .default_value("restore"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .help("Run in dry-run mode (simulate operations without executing)")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    let dry_run_mode = matches.get_flag("dry-run");
    let operation_mode = match matches.get_one::<String>("operation").map(|s| s.as_str()) {
        Some("create-backup") => OperationMode::CreateBackup,
        _ => OperationMode::Restore,
    };

    // Run the application with the selected mode
    run_tui_app(dry_run_mode, operation_mode).await?;

    Ok(())
}

async fn run_tui_app(dry_run_mode: bool, operation_mode: OperationMode) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let app = App::new(dry_run_mode, operation_mode).await?;
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
