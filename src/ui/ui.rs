use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};

use crate::ui::app::App;

pub fn draw_ui<B: Backend>(f: &mut Frame<B>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(f.size());

    draw_header(f, chunks[0], &app.current_node.path);
    draw_file_list(f, chunks[1], app);
    draw_status_bar(f, chunks[2], app);
}

fn draw_header<B: Backend>(f: &mut Frame<B>, area: Rect, current_path: &std::path::Path) {
    let header = Block::default()
        .borders(Borders::ALL)
        .title(" Disk Usage Analyzer (q to quit)");

    let path_text = Paragraph::new(current_path.display().to_string())
        .block(Block::default().borders(Borders::BOTTOM));

    f.render_widget(header, area);
    f.render_widget(path_text, area);
}

fn draw_file_list<B: Backend>(f: &mut Frame<B>, area: Rect, app: &App) {
    let header_cells = ["Name", "Size"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().add_modifier(Modifier::BOLD)));

    let header = Row::new(header_cells)
        .style(Style::default().add_modifier(Modifier::REVERSED))
        .bottom_margin(1);

    let items: Vec<Row> = app
        .current_node
        .children
        .iter()
        .enumerate()
        .map(|(i, child)| {
            let is_selected = app.selected == i;
            let style = if is_selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };

            let name = child
                .path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "/".to_string());

            let name_style = if child.is_dir {
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            Row::new(vec![
                name,
                humansize::format_size(child.size, humansize::DECIMAL),
            ])
            .style(style)
            .style(name_style)
        })
        .collect();

    let table = Table::new(items)
        .header(header)
        .block(Block::default().borders(Borders::ALL))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .widths(&[Constraint::Percentage(70), Constraint::Percentage(30)]);

    let mut state = TableState::default();
    state.select(Some(app.selected));
    f.render_stateful_widget(table, area, &mut state);
}

fn draw_status_bar<B: Backend>(f: &mut Frame<B>, area: Rect, app: &App) {
    let (file_count, dir_count) =
        app.current_node
            .children
            .iter()
            .fold((0, 0), |(files, dirs), child| {
                if child.is_dir {
                    (files, dirs + 1)
                } else {
                    (files + 1, dirs)
                }
            });

    let status = format!(
        "↑/k/↓/j: Navigate | →/Enter: Open | ←/Backspace: Go Back | s: Toggle Sort | Files: {} | Dirs: {} | Total: {}",
        file_count,
        dir_count,
        humansize::format_size(app.current_node.size, humansize::DECIMAL)
    );

    let status_bar =
        Paragraph::new(Span::raw(status)).block(Block::default().borders(Borders::ALL));

    f.render_widget(status_bar, area);
}
