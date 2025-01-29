use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{
        List, ListDirection, ListState, Scrollbar, ScrollbarOrientation, ScrollbarState,
        StatefulWidget,
    },
};

use crate::{application::Application, launcher::LauncherState};

pub struct ApplicationList;

impl StatefulWidget for ApplicationList {
    type State = LauncherState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let [_, scrollbar_area] =
            Layout::horizontal([Constraint::Min(1), Constraint::Max(1)]).areas(area);

        let mut highlighted_and_filtered_applications = Vec::new();
        for application in &state.application_list.filtered_applications {
            let mut highlighted_name = vec![Span::from(" ")];
            if state.config.application_list.display_icons {
                highlighted_name.push(application.get_icon());
            }
            highlighted_name
                .append(&mut application.get_highlighted_name(state.input.filter.as_str()));
            highlighted_and_filtered_applications.push(Line::from(highlighted_name));
        }

        let list = List::new(highlighted_and_filtered_applications)
            .style(Style::new().fg(Color::White))
            .highlight_style(Style::new().fg(Color::Cyan).bg(Color::Black).not_reversed())
            .direction(ListDirection::TopToBottom);

        let list_state = &mut state.application_list.list_state;
        if let None = list_state.selected() {
            list_state.select_first();
        }

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_symbol(None)
            .thumb_symbol("┃")
            .style(Style::new().fg(Color::White));
        let scrollable_range =
            (state.application_list.filtered_applications.len() as i16 - area.height as i16 + 3)
                .max(0);
        let mut scrollbar_state = state
            .application_list
            .scrollbar_state
            .content_length(scrollable_range as usize)
            .position(list_state.offset());

        StatefulWidget::render(list, area, buf, list_state);
        StatefulWidget::render(scrollbar, scrollbar_area, buf, &mut scrollbar_state);
    }
}

#[derive(Debug)]
pub struct ApplicationListState {
    pub filtered_applications: Vec<Application>,
    pub applications: Vec<Application>,
    list_state: ListState,
    scrollbar_state: ScrollbarState,
}

impl ApplicationListState {
    pub fn update(&mut self, filter: &str) {
        let mut filtered_applications = self
            .applications
            .clone()
            .into_iter()
            .filter(|entry| entry.name.to_lowercase().contains(&filter.to_lowercase()))
            .collect::<Vec<Application>>();
        filtered_applications.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        self.filtered_applications = filtered_applications;
    }

    pub fn selected(&self) -> Option<&Application> {
        let Some(i) = self.list_state.selected() else {
            return None;
        };
        Some(&self.filtered_applications[i])
    }

    pub fn select_previous(&mut self) {
        self.list_state.select_previous();
    }

    pub fn select_next(&mut self) {
        self.list_state.select_next();
    }
}

impl Default for ApplicationListState {
    fn default() -> Self {
        let applications = Application::find_all();
        Self {
            filtered_applications: applications.clone(),
            applications,
            list_state: ListState::default(),
            scrollbar_state: ScrollbarState::default(),
        }
    }
}
