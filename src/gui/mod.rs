// gui/mod.rs
// generated as app.rs using orhun/rust-tui-template via cargo-generate
// Mar 15 2023
use std::error;
use ::tui::backend::Backend;
use ::tui::layout::{Alignment, Rect};
use ::tui::style::{Color, Style};
use ::tui::terminal::Frame;
use ::tui::widgets::{Block, BorderType, Borders, Paragraph};
pub mod tui;
pub mod handler;
pub mod event;
pub mod viewport;
use viewport::Viewport;

/// Application result type.
pub type AppResult<T> = std::result::Result<T, Box<dyn error::Error>>;

/// Application.
#[derive(Debug)]
pub struct App {
    /// Is the application running?
    pub running: bool,
}

impl Default for App {
    fn default() -> Self {
        Self { running: true }
    }
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Handles the tick event of the terminal.
    pub fn tick(&self) {}

    /// Renders the user interface widgets.
    pub fn render<B: Backend>(&mut self, frame: &mut Frame<'_, B>) {
        // This is where you add new widgets.
        // See the following resources:
        // - https://docs.rs/tui/latest/tui/widgets/index.html
        // - https://github.com/fdehau/tui-rs/tree/master/examples
/*        frame.render_widget(
            Paragraph::new(
                "This is a tui-rs template.\nPress `Esc`, `Ctrl-C` or `q` to stop running.",
            )
            .block(
                Block::default()
                    .title("Template")
                    .title_alignment(Alignment::Center)
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .style(Style::default().fg(Color::Cyan).bg(Color::Black))
            .alignment(Alignment::Center),
            frame.size(),
        );
*/
	    let mid_y = (frame.size().top() + frame.size().bottom() ) / 2;
	    frame.render_widget(
            Viewport::new("This is the replacement text")
	        .block(
		        Block::default()
			        .title("test")
			        .title_alignment(Alignment::Left)
			        .borders(Borders::ALL)
			        .border_type(BorderType::Double)
			        .border_style(Style::default().fg(Color::White)),
			)
			.style(Style::default().fg(Color::Cyan).bg(Color::White))
			.alignment(Alignment::Center),
			Rect::new(0, mid_y, frame.size().width, frame.size().height/2),
        );
    }
}

// EOF
