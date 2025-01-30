use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Position, Rect},
    style::{Color, Style, Stylize},
    widgets::{Paragraph, StatefulWidget, Widget},
};

use crate::launcher::LauncherState;

pub struct Input;

impl StatefulWidget for Input {
    type State = LauncherState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let [icon_area, input_area] =
            Layout::horizontal([Constraint::Length(3), Constraint::Min(1)]).areas(area);
        let icon = Paragraph::new("");
        state.input.width = area.width as usize;
        let input_text;
        if state.input.filter.len() == 0 {
            input_text = Paragraph::new(state.config.input.placeholder.as_str())
                .style(Style::new().fg(Color::DarkGray).italic());
        } else {
            let len = state
                .input
                .filter
                .len()
                .min(state.input.width + state.input.overflow);
            let filter_text_to_display = &state.input.filter[state.input.overflow..len];
            input_text =
                Paragraph::new(filter_text_to_display).style(Style::new().fg(Color::White));
        }
        Widget::render(icon, icon_area, buf);
        Widget::render(input_text, input_area, buf);
    }
}

#[derive(Debug)]
pub struct InputState {
    pub filter: String,
    pub cursor_index: usize,
    overflow: usize,
    width: usize,
}

impl InputState {
    pub fn get_cursor_position(&self) -> Position {
        Position::new(self.cursor_index as u16 + 5, 1)
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_index == 0 && self.overflow > 0 {
            self.overflow = self.overflow.saturating_sub(1);
        }
        let cursor_moved_left = self.cursor_index.saturating_sub(1);
        self.cursor_index = self.clamp_cursor(cursor_moved_left);
    }

    pub fn move_cursor_right(&mut self) {
        let max_overflow = self.filter.len().saturating_sub(self.width);
        if self.cursor_index == self.width && self.overflow < max_overflow {
            self.overflow = self.overflow.saturating_add(1);
        }
        let cursor_moved_right = self.cursor_index.saturating_add(1);
        self.cursor_index = self.clamp_cursor(cursor_moved_right);
    }

    pub fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.filter.insert(index, new_char);
        self.set_overflow();
        if self.overflow == 0 {
            self.move_cursor_right();
        }
    }

    pub fn delete_char(&mut self) {
        let is_cursor_leftmost = self.cursor_index + self.overflow == 0;
        if is_cursor_leftmost {
            return;
        }
        let current_index = self.cursor_index + self.overflow;
        let from_left_to_current_index = current_index - 1;
        let before_char_to_delete = self.filter.chars().take(from_left_to_current_index);
        let after_char_to_delete = self.filter.chars().skip(current_index);
        self.filter = before_char_to_delete.chain(after_char_to_delete).collect();
        if self.overflow == 0 {
            self.move_cursor_left();
        }
        self.set_overflow();
    }

    pub fn right_delete_char(&mut self) {
        let is_cursor_rightmost = self.cursor_index + self.overflow == self.filter.len();
        if is_cursor_rightmost {
            return;
        }
        let index = self.cursor_index + self.overflow;
        if self.overflow > 0 {
            self.move_cursor_right();
        }
        self.filter.remove(index);
        self.set_overflow();
    }

    fn set_overflow(&mut self) {
        self.overflow = self.filter.len().saturating_sub(self.width);
    }

    fn byte_index(&self) -> usize {
        self.filter
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.cursor_index + self.overflow)
            .unwrap_or(self.filter.len())
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        let max = self.filter.chars().count().min(self.width);
        new_cursor_pos.clamp(0, max)
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            filter: String::new(),
            cursor_index: 0,
            width: 32,
            overflow: 0,
        }
    }
}
