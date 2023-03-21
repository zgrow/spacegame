// app/handler.rs
// generated from orhun/rust-tui-template via cargo-generate
// Mar 15 2023
use crate::app::{AppResult, GameEngine};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Handles the key events and updates the state of [`GameEngine`].
pub fn handle_key_events(key_event: KeyEvent, app: &mut GameEngine) -> AppResult<()> {
	match key_event.code {
		// Exit application on `ESC` or `q`
		KeyCode::Esc | KeyCode::Char('q') => {
			app.running = false;
		}
		// Exit application on `Ctrl-C`
		KeyCode::Char('c') | KeyCode::Char('C') => {
			if key_event.modifiers == KeyModifiers::CONTROL {
				app.running = false;
			}
		}
		// Other handlers you could add here.
		_ => {}
	}
	Ok(())
}

// EOF
