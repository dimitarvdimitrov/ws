use crate::tui::app::ConfirmDialog;
use ratatui::{prelude::*, widgets::*};

pub fn render_dialog(f: &mut Frame, dialog: &ConfirmDialog) {
    let area = f.area();

    // Calculate dialog size and position (centered)
    let dialog_width = 50.min(area.width.saturating_sub(4));
    let dialog_height = 7.min(area.height.saturating_sub(4));

    let x = (area.width.saturating_sub(dialog_width)) / 2;
    let y = (area.height.saturating_sub(dialog_height)) / 2;

    let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

    // Clear area behind dialog
    f.render_widget(Clear, dialog_area);

    // Dialog box
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(" Confirm ");

    let inner = block.inner(dialog_area);
    f.render_widget(block, dialog_area);

    // Message and buttons
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let message = Paragraph::new(dialog.message.as_str())
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Center);
    f.render_widget(message, chunks[0]);

    let buttons = Paragraph::new("[Y]es  [N]o")
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center);
    f.render_widget(buttons, chunks[1]);
}
