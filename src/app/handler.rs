// app/handler.rs
// generated from orhun/rust-tui-template via cargo-generate
// Mar 15 2023
use crate::app::{AppResult, GameEngine};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::components::*;
use crate::components::Direction;
use crate::components::GameEvent::PlayerMove;
use bevy::ecs::event::Events;

pub fn key_parser(key_event: KeyEvent, eng: &mut GameEngine) -> AppResult<()> {
	let game_events: &mut Events<TuiEvent> = &mut eng.app.world.get_resource_mut::<Events<TuiEvent>>().unwrap();
	match key_event.code {
		// Exit englication on `ESC` or `q`
		KeyCode::Esc | KeyCode::Char('q') => {
			eng.running = false;
		}
		// Exit englication on `Ctrl-C`
		KeyCode::Char('c') | KeyCode::Char('C') => {
			if key_event.modifiers == KeyModifiers::CONTROL {
				eng.running = false;
			}
		}
		// Move player
		KeyCode::Char('h') | KeyCode::Left => {
			game_events.send(TuiEvent{etype: PlayerMove(Direction::W)});
		}
		KeyCode::Char('l') | KeyCode::Right => {
			game_events.send(TuiEvent{etype: PlayerMove(Direction::E)});
		}
		KeyCode::Char('j') | KeyCode::Down => {
			game_events.send(TuiEvent{etype: PlayerMove(Direction::S)});
		}
		KeyCode::Char('k') | KeyCode::Up => {
			game_events.send(TuiEvent{etype: PlayerMove(Direction::N)});
		}
		KeyCode::Char('y') => {
			game_events.send(TuiEvent{etype: PlayerMove(Direction::NW)});
		}
		KeyCode::Char('u') => {
			game_events.send(TuiEvent{etype: PlayerMove(Direction::NE)});
		}
		KeyCode::Char('b') => {
			game_events.send(TuiEvent{etype: PlayerMove(Direction::SW)});
		}
		KeyCode::Char('n') => {
			game_events.send(TuiEvent{etype: PlayerMove(Direction::SE)});
		}
		// Other handlers you could add here.
		_ => {}
	}
	Ok(())
}

// EOF
