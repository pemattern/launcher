use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use nix::{
    sys::{
        signal::{kill, Signal},
        wait::{waitpid, WaitPidFlag, WaitStatus},
    },
    unistd::{execvp, fork, getppid, ForkResult, Pid},
};
use procfs::process::Process;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Margin, Position, Rect},
    widgets::{Block, StatefulWidget, Widget},
    DefaultTerminal, Frame,
};
use std::{
    io::{self},
    process::exit,
    sync::mpsc,
    thread,
    time::Duration,
};

use crate::{
    config::Config,
    widgets::{
        application_list::{ApplicationList, ApplicationListState},
        counter::{Counter, CounterState},
        divider::{Divider, DividerState},
        input::{Input, InputState},
    },
};

#[derive(Debug)]
pub struct Launcher {
    mode: RunMode,
    state: LauncherState,
}

#[derive(Debug, Default)]
enum RunMode {
    #[default]
    Running,
    Exit,
}

impl Launcher {
    pub fn new(config: Config) -> Self {
        Self {
            mode: RunMode::Running,
            state: LauncherState::from_config(config),
        }
    }

    pub fn run(
        &mut self,
        terminal: &mut DefaultTerminal,
        receiver: mpsc::Receiver<()>,
    ) -> io::Result<()> {
        loop {
            match &self.mode {
                RunMode::Running => {
                    if receiver.try_recv() == Ok(()) {
                        terminal.clear().unwrap();
                        self.state.reload_config(Config::load());
                    }
                    terminal.draw(|frame| self.draw(frame))?;
                    self.handle_input()?;
                }
                RunMode::Exit => break,
            }
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let index = self.state.input.cursor_index as u16;
        frame.render_widget(self, frame.area());
        frame.set_cursor_position(Position::new(index + 2, 1));
    }

    fn select_application(&mut self) {
        let Some(application) = self.state.application_list.selected() else {
            return;
        };
        if application.terminal {
            ratatui::restore();
            let _ = execvp(&application.filename, application.args.as_slice());
            return;
        }
        let shell_pid = getppid();
        let terminal_pid = Process::new(shell_pid.as_raw())
            .unwrap()
            .stat()
            .unwrap()
            .ppid;
        match unsafe { fork() } {
            Ok(ForkResult::Parent { child }) => loop {
                match waitpid(child, Some(WaitPidFlag::WNOHANG)) {
                    Ok(WaitStatus::StillAlive) => {
                        let _ = kill(Pid::from_raw(terminal_pid), Signal::SIGTERM);
                        exit(0)
                    }
                    Err(_) => todo!(),
                    _ => {
                        thread::sleep(Duration::from_millis(10));
                    }
                }
            },
            Ok(ForkResult::Child) => {
                let _ = execvp(&application.filename, application.args.as_slice());
            }
            Err(_) => todo!(),
        }
    }

    fn handle_input(&mut self) -> io::Result<()> {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char(to_insert) => self.state.input.enter_char(to_insert),
                    KeyCode::Backspace => self.state.input.delete_char(),
                    KeyCode::Delete => self.state.input.right_delete_char(),
                    KeyCode::Left => self.state.input.move_cursor_left(),
                    KeyCode::Right => self.state.input.move_cursor_right(),
                    KeyCode::Enter => self.select_application(),
                    KeyCode::Down | KeyCode::Tab => self.state.application_list.select_next(),
                    KeyCode::Up | KeyCode::BackTab => self.state.application_list.select_previous(),
                    KeyCode::Esc => self.mode = RunMode::Exit,
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

impl Widget for &mut Launcher {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let main_block = Block::bordered();
        Widget::render(main_block, area, buf);

        let [filter_and_counter_area, divider_area, list_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .areas(area.inner(Margin::new(1, 1)));
        let counter_text = self.state.application_list.get_counter_text();
        let [filter_area, _, counter_area] = Layout::horizontal([
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(counter_text.len() as u16),
        ])
        .areas(filter_and_counter_area.inner(Margin::new(1, 0)));
        StatefulWidget::render(Input, filter_area, buf, &mut self.state);
        StatefulWidget::render(Divider, divider_area, buf, &mut self.state);
        StatefulWidget::render(Counter, counter_area, buf, &mut self.state);
        StatefulWidget::render(ApplicationList, list_area, buf, &mut self.state);
    }
}

#[derive(Debug)]
pub struct LauncherState {
    pub config: Config,
    pub input: InputState,
    pub counter: CounterState,
    pub divider: DividerState,
    pub application_list: ApplicationListState,
}

impl LauncherState {
    pub fn from_config(config: Config) -> Self {
        Self {
            config,
            input: InputState::default(),
            counter: CounterState::default(),
            divider: DividerState::default(),
            application_list: ApplicationListState::default(),
        }
    }

    fn reload_config(&mut self, config: Config) {
        self.config = config;
    }
}
