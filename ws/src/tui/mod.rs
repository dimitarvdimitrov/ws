mod app;
mod confirmation;
mod tree;

use crate::config::Config;
use crate::db::Database;
use app::App;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{prelude::*, widgets::*};
use std::error::Error;
use std::io;

pub fn run(db: Database, config: Config, filter: String) -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new(db, config, filter)?;

    // Main loop
    loop {
        terminal.draw(|f| ui(f, &app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                // Handle Ctrl+C to quit
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    break;
                }

                match app.handle_key(key.code) {
                    app::Action::Continue => {}
                    app::Action::Launch => {
                        // Restore terminal before launching
                        disable_raw_mode()?;
                        execute!(
                            terminal.backend_mut(),
                            LeaveAlternateScreen,
                            DisableMouseCapture
                        )?;
                        terminal.show_cursor()?;

                        app.launch_selection()?;
                        return Ok(());
                    }
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Filter input
            Constraint::Min(1),    // Tree view
            Constraint::Length(2), // Help bar
        ])
        .split(f.area());

    // Filter input
    let filter_block = Block::default()
        .borders(Borders::ALL)
        .title(" ws - worktree manager ");

    let filter_text = Paragraph::new(format!("Filter: {}", app.filter))
        .block(filter_block)
        .style(Style::default());
    f.render_widget(filter_text, chunks[0]);

    // Tree view
    let tree_block = Block::default().borders(Borders::ALL);
    let inner_area = tree_block.inner(chunks[1]);
    f.render_widget(tree_block, chunks[1]);

    tree::render_tree(f, inner_area, app);

    // Help bar
    let help_text = if app.confirm_dialog.is_some() {
        " y/n confirm  Esc cancel "
    } else {
        " ↑↓ navigate  ←→ switch worktree  Space select session  Enter expand/launch  Ctrl+C quit "
    };
    let help = Paragraph::new(help_text).style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, chunks[2]);

    // Confirmation dialog overlay
    if let Some(ref dialog) = app.confirm_dialog {
        confirmation::render_dialog(f, dialog);
    }
}
