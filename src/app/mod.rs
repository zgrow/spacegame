// app/mod.rs
// generated as app.rs using orhun/rust-tui-template via cargo-generate
// Mar 15 2023
// Here, ::tui refers to tui-rs, while the 'pub mod tui' below is located at app/tui.rs
use std::error;
use bevy::app::App;
use ::tui::backend::Backend;
use ::tui::layout::{Alignment, Rect};
use ::tui::style::{Color, Style};
use ::tui::terminal::Frame;
use ::tui::widgets::{Block, BorderType, Borders};
//use bevy::prelude::*;
pub mod handler;
pub mod event;
pub mod viewport;
pub mod tui;
use viewport::Viewport;
use super::map::Map;

/// Application result type.
pub type AppResult<T> = std::result::Result<T, Box<dyn error::Error>>;
#[derive(Debug)]
pub struct GameEngine {
	/// Is the application running?
	pub running: bool,
	pub app: App, // bevy::app::App, contains all of the ECS bits
}
impl GameEngine {
	/// Constructs a new instance of [`GameEngine`].
	pub fn new() -> Self {
		Self {
			running: true,
			app: App::new(),
		}
	}
	/// Runs a single update cycle of the game state
	pub fn tick(&self) {
		eprintln!("TICK");
	}
	/// Renders the user interface widgets.
	pub fn render<B: Backend>(&mut self, frame: &mut Frame<'_, B>) {
		// This is where you add new widgets.
		// See the following resources:
		// - https://docs.rs/tui/latest/tui/widgets/index.html
		// - https://github.com/fdehau/tui-rs/tree/master/examples
		frame.render_widget(
			Viewport::new(&self.app)
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
			Rect::new(0, 0, frame.size().width, frame.size().height),
		);
	}
}

// EOF
