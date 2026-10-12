//! The terminal event loop: raw-mode setup, input polling, and teardown.

use std::io::{self, Write};
use std::time::Duration;

use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::mpsc::UnboundedSender;

use super::{App, MOUSE_OFF, MOUSE_ON, PASTE_OFF, PASTE_ON};
use crate::agent::AgentEvent;

impl App {
    /// Run the event loop until the user quits.
    ///
    /// # Errors
    /// Returns terminal IO errors.
    pub fn run<F>(&mut self, mut on_submit: F) -> io::Result<()>
    where
        F: FnMut(String, UnboundedSender<AgentEvent>) + Send + 'static,
    {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        write!(stdout, "{MOUSE_ON}{PASTE_ON}")?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let res = self.event_loop(&mut terminal, &mut on_submit);

        disable_raw_mode()?;
        write!(terminal.backend_mut(), "{MOUSE_OFF}{PASTE_OFF}")?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;
        res
    }

    fn event_loop<B, F>(&mut self, terminal: &mut Terminal<B>, on_submit: &mut F) -> io::Result<()>
    where
        B: ratatui::backend::Backend,
        F: FnMut(String, UnboundedSender<AgentEvent>) + Send + 'static,
    {
        loop {
            self.tick = self.tick.wrapping_add(1);
            terminal.draw(|f| self.draw(f))?;

            // Drain agent events without blocking.
            let mut pending = Vec::new();
            if let Some(rx) = &mut self.events {
                while let Ok(ev) = rx.try_recv() {
                    pending.push(ev);
                }
            }
            for ev in pending {
                if let Some(next) = self.apply(ev) {
                    self.submit(next, on_submit);
                }
            }

            if event::poll(Duration::from_millis(50))? {
                let before = self.mouse_capture;
                match event::read()? {
                    Event::Key(key) => {
                        if self.handle_key(key, on_submit) {
                            return Ok(());
                        }
                    }
                    Event::Resize(_, _) => {}
                    Event::Mouse(m) => self.handle_mouse(m),
                    Event::Paste(s) => self.input.push_str(&s),
                    _ => {}
                }
                if before != self.mouse_capture {
                    let mut out = io::stdout();
                    if self.mouse_capture {
                        write!(out, "{MOUSE_ON}")?;
                    } else {
                        write!(out, "{MOUSE_OFF}")?;
                    }
                }
            }
        }
    }
}
