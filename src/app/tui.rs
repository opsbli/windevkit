use std::io::{self, Stdout};
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use crate::app::{self, AppEntry};

const CATEGORIES: [&str; 7] = [
    "All", "Browser", "IDE", "Runtime", "Dev Tool", "Utility", "Other",
];

pub fn run(apps: Vec<AppEntry>) -> Result<Vec<AppEntry>> {
    let mut tui = AppTui::new(apps);
    tui.run()
}

struct AppTui {
    apps: Vec<AppEntry>,
    list_state: ListState,
    search: String,
    search_mode: bool,
    category_idx: usize,
    selected_only: bool,
}

impl AppTui {
    fn new(apps: Vec<AppEntry>) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            apps,
            list_state,
            search: String::new(),
            search_mode: false,
            category_idx: 0,
            selected_only: false,
        }
    }

    fn run(&mut self) -> Result<Vec<AppEntry>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let res = self.event_loop(&mut terminal);

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        res
    }

    fn event_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<Vec<AppEntry>> {
        loop {
            terminal.draw(|f| self.render(f))?;

            if event::poll(Duration::from_millis(200))?
                && let Event::Key(key) = event::read()?
            {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                if self.search_mode {
                    match key.code {
                        KeyCode::Esc => self.search_mode = false,
                        KeyCode::Enter => self.search_mode = false,
                        KeyCode::Backspace => {
                            self.search.pop();
                            self.ensure_selected_valid();
                        }
                        KeyCode::Char(c) => {
                            self.search.push(c);
                            self.ensure_selected_valid();
                        }
                        _ => {}
                    }
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(Vec::new()),
                    KeyCode::Char('/') => self.search_mode = true,
                    KeyCode::Down | KeyCode::Char('j') => self.next(),
                    KeyCode::Up | KeyCode::Char('k') => self.previous(),
                    KeyCode::Char(' ') => self.toggle_current(),
                    KeyCode::Char('a') => self.toggle_all_visible(),
                    KeyCode::Char('c') => {
                        self.category_idx = (self.category_idx + 1) % CATEGORIES.len();
                        self.ensure_selected_valid();
                    }
                    KeyCode::Char('s') => {
                        self.selected_only = !self.selected_only;
                        self.ensure_selected_valid();
                    }
                    KeyCode::Enter => {
                        return Ok(self.apps.iter().filter(|a| a.selected).cloned().collect());
                    }
                    _ => {}
                }
            }
        }
    }

    fn render(&mut self, f: &mut ratatui::Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(2),
            ])
            .split(f.area());

        let title = Paragraph::new("windevkit App TUI — space toggle | a all | c category | / search | s selected | enter confirm | q quit")
            .block(Block::default().borders(Borders::ALL).title("Help"));
        f.render_widget(title, chunks[0]);

        let status =
            Paragraph::new(vec![Line::from(vec![
                Span::styled("Category: ", Style::default().fg(Color::Yellow)),
                Span::raw(CATEGORIES[self.category_idx]),
                Span::raw("    "),
                Span::styled("Search: ", Style::default().fg(Color::Yellow)),
                Span::raw(if self.search.is_empty() {
                    "<none>"
                } else {
                    &self.search
                }),
                Span::raw("    "),
                Span::styled("Selected only: ", Style::default().fg(Color::Yellow)),
                Span::raw(if self.selected_only { "on" } else { "off" }),
            ])])
            .block(Block::default().borders(Borders::ALL).title(
                if self.search_mode {
                    "Search mode"
                } else {
                    "Status"
                },
            ));
        f.render_widget(status, chunks[1]);

        let visible = self.visible_indices();
        let items: Vec<ListItem> = visible
            .iter()
            .map(|&idx| {
                let app = &self.apps[idx];
                let mark = if app.selected { "☑" } else { "☐" };
                let category = app::category_for_app(app);
                ListItem::new(format!(
                    "{} [{}] {} v{}",
                    mark, category, app.name, app.version
                ))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Applications"))
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");
        f.render_stateful_widget(list, chunks[2], &mut self.list_state);

        let selected_count = self.apps.iter().filter(|a| a.selected).count();
        let footer = Paragraph::new(format!(
            "Visible: {}    Selected: {}    Total: {}",
            visible.len(),
            selected_count,
            self.apps.len()
        ))
        .block(Block::default().borders(Borders::ALL));
        f.render_widget(footer, chunks[3]);
    }

    fn visible_indices(&self) -> Vec<usize> {
        self.apps
            .iter()
            .enumerate()
            .filter(|(_, app)| {
                let category_ok = self.category_idx == 0
                    || app::category_for_app(app) == CATEGORIES[self.category_idx];
                let search_ok = self.search.is_empty()
                    || app
                        .name
                        .to_lowercase()
                        .contains(&self.search.to_lowercase())
                    || app.id.to_lowercase().contains(&self.search.to_lowercase());
                let selected_ok = !self.selected_only || app.selected;
                category_ok && search_ok && selected_ok
            })
            .map(|(i, _)| i)
            .collect()
    }

    fn ensure_selected_valid(&mut self) {
        let visible = self.visible_indices();
        if visible.is_empty() {
            self.list_state.select(None);
        } else {
            let current = self.list_state.selected().unwrap_or(0);
            let new_idx = current.min(visible.len().saturating_sub(1));
            self.list_state.select(Some(new_idx));
        }
    }

    fn next(&mut self) {
        let visible = self.visible_indices();
        if visible.is_empty() {
            self.list_state.select(None);
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => (i + 1) % visible.len(),
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn previous(&mut self) {
        let visible = self.visible_indices();
        if visible.is_empty() {
            self.list_state.select(None);
            return;
        }
        let i = match self.list_state.selected() {
            Some(0) | None => visible.len() - 1,
            Some(i) => i - 1,
        };
        self.list_state.select(Some(i));
    }

    fn toggle_current(&mut self) {
        let visible = self.visible_indices();
        let Some(sel) = self.list_state.selected() else {
            return;
        };
        let Some(&idx) = visible.get(sel) else {
            return;
        };
        self.apps[idx].selected = !self.apps[idx].selected;
    }

    fn toggle_all_visible(&mut self) {
        let visible = self.visible_indices();
        if visible.is_empty() {
            return;
        }
        let all_selected = visible.iter().all(|&i| self.apps[i].selected);
        for idx in visible {
            self.apps[idx].selected = !all_selected;
        }
    }
}
