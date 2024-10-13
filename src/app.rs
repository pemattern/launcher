use std::{
    env,
    fs::{self},
    io::{self},
    os::unix::process::CommandExt,
    path::Path,
    process::Command,
};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use fork::{fork, Fork};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Position, Rect},
    style::{Color, Style},
    widgets::{
        Block, List, ListDirection, ListState, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, StatefulWidget, Widget,
    },
    DefaultTerminal, Frame,
};

use crate::desktop_entry::DesktopEntry;

#[derive(Debug)]
pub struct App {
    filter: String,
    character_index: usize,
    desktop_apps: Vec<DesktopEntry>,
    filtered_desktop_apps: Vec<DesktopEntry>,
    list_state: ListState,
    scrollbar_state: ScrollbarState,
    should_exit: bool,
}

impl App {
    pub fn new() -> Self {
        let desktop_apps = Self::get_desktop_apps();
        Self {
            filter: String::new(),
            character_index: 0,
            desktop_apps: desktop_apps.clone(),
            filtered_desktop_apps: desktop_apps.clone(),
            list_state: ListState::default(),
            scrollbar_state: ScrollbarState::new(desktop_apps.len()).position(0),
            should_exit: false,
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.should_exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let index = self.character_index.clone();
        frame.render_widget(self, frame.area());
        frame.set_cursor_position(Position::new(index as u16 + 1, 1));
    }

    fn handle_events(&mut self) -> io::Result<()> {
        if let Event::Key(key) = event::read()? {
            if key.modifiers == KeyModifiers::CONTROL {
                match key {
                    KeyEvent {
                        code: KeyCode::Char('k'),
                        ..
                    } => self.list_state.select_previous(),
                    KeyEvent {
                        code: KeyCode::Char('j'),
                        ..
                    } => self.list_state.select_next(),
                    _ => {}
                }
            } else if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Esc => self.should_exit = true,
                    KeyCode::Enter => self.select_app(),
                    KeyCode::Char(to_insert) => self.enter_char(to_insert),
                    KeyCode::Backspace => self.delete_char(),
                    KeyCode::Delete => self.right_delete_char(),
                    KeyCode::Left => self.move_cursor_left(),
                    KeyCode::Right => self.move_cursor_right(),
                    KeyCode::Up => self.list_state.select_previous(),
                    KeyCode::Down => self.list_state.select_next(),
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn select_app(&mut self) {
        if let Some(i) = self.list_state.selected() {
            let app = &self.filtered_desktop_apps[i];
            let shell = env::var("SHELL").expect("unable to read $SHELL env");
            if app.terminal {
                ratatui::restore();
                let _ = Command::new(&app.exec).exec();
            } else {
                let output = Command::new(&shell)
                    .args(&[
                        "-c",
                        format!("ps -o ppid= -p {}", std::process::id()).as_str(),
                    ])
                    .output()
                    .expect("unable to get ppid");
                match fork() {
                    Ok(Fork::Child) => {
                        let ppid = String::from_utf8_lossy(&output.stdout);
                        let _ = Command::new(&shell)
                            .args(&["-c", "sleep .1"])
                            .output()
                            .expect("...");
                        ratatui::restore();
                        let _ = Command::new(&shell)
                            .args(&["-c", format!("kill -9 {}", ppid).as_str()])
                            .status()
                            .expect("unable to kill terminal process");
                    }
                    Ok(Fork::Parent(_)) => {
                        let _ = Command::new(&shell)
                            .args(&["-c", format!("{} & disown", &app.exec).as_str()])
                            .exec();
                    }
                    Err(_) => panic!("fork failed"),
                }
            }
        }
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved_right = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_left = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }

    fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.filter.insert(index, new_char);
        self.move_cursor_right();
    }

    fn byte_index(&self) -> usize {
        self.filter
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.filter.len())
    }

    fn delete_char(&mut self) {
        let is_cursor_leftmost = self.character_index == 0;
        if is_cursor_leftmost {
            return;
        }
        let current_index = self.character_index;
        let from_left_to_current_index = current_index - 1;
        let before_char_to_delete = self.filter.chars().take(from_left_to_current_index);
        let after_char_to_delete = self.filter.chars().skip(current_index);
        self.filter = before_char_to_delete.chain(after_char_to_delete).collect();
        self.move_cursor_left();
    }

    fn right_delete_char(&mut self) {
        let is_cursor_rightmost = self.character_index == self.filter.len();
        if is_cursor_rightmost {
            return;
        }
        let cursor_index = self.character_index;
        self.filter.remove(cursor_index);
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.filter.chars().count())
    }

    fn get_desktop_apps() -> Vec<DesktopEntry> {
        let mut apps = Vec::new();
        let home = env::var("HOME").expect("unable to read $HOME env");
        let dirs = vec![
            "/usr/share/applications/".to_string(),
            format!("{}/.local/share/applications/", home),
        ];
        for dir in dirs {
            let path = Path::new(&dir);
            if path.exists() && path.is_dir() {
                for entry in fs::read_dir(path).expect("unable to read target directory") {
                    let entry = entry.expect("unable to read entry");
                    let path = entry.path();
                    if path.is_file()
                        && path.extension().and_then(|s| s.to_str()) == Some("desktop")
                    {
                        match DesktopEntry::from_file(path.to_str().unwrap()) {
                            Some(app) => apps.push(app),
                            None => continue,
                        }
                    }
                }
            }
        }
        apps
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [input_area, list_area] =
            Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).areas(area);
        let [_, scrollbar_area] = Layout::horizontal([Constraint::Min(1), Constraint::Max(1)])
            .margin(1)
            .areas(list_area);

        let input = Paragraph::new(self.filter.as_str()).block(Block::bordered().title("Filter"));

        self.filtered_desktop_apps = self
            .desktop_apps
            .clone()
            .into_iter()
            .filter(|app| {
                app.name
                    .to_lowercase()
                    .contains(&self.filter.to_lowercase())
            })
            .collect();

        self.filtered_desktop_apps
            .sort_by(|a, b| a.name.cmp(&b.name));

        let list = List::new(
            self.filtered_desktop_apps
                .iter()
                .map(|app| format!(" {} {}", app.icon.clone(), app.name.clone()))
                .collect::<Vec<String>>(),
        )
        .block(Block::bordered().title("Apps"))
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Black).bg(Color::White))
        .direction(ListDirection::TopToBottom);

        if let None = self.list_state.selected() {
            self.list_state.select_first();
        }

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_symbol(None)
            .thumb_symbol("┃");

        let scrollable_range =
            (self.filtered_desktop_apps.len() as i16 - list_area.height as i16 + 3).max(0);

        self.scrollbar_state = self
            .scrollbar_state
            .content_length(scrollable_range as usize)
            .position(self.list_state.offset());

        Widget::render(input, input_area, buf);
        StatefulWidget::render(list, list_area, buf, &mut self.list_state);
        StatefulWidget::render(scrollbar, scrollbar_area, buf, &mut self.scrollbar_state);
    }
}