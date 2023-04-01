// app/handler.rs
// generated from orhun/rust-tui-template via cargo-generate
// Mar 15 2023
use crate::app::{AppResult, GameEngine, MainMenuItems};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::components::*;
use crate::components::Direction;
use crate::components::GameEvent::PlayerMove;
use bevy::ecs::event::Events;

pub fn key_parser(key_event: KeyEvent, eng: &mut GameEngine) -> AppResult<()> {
	let game_events: &mut Events<TuiEvent> = &mut eng.app.world.get_resource_mut::<Events<TuiEvent>>().unwrap();
	// parse any debug/overrride controls
	if (key_event.code == KeyCode::Char('c') || key_event.code == KeyCode::Char('C'))
	&& key_event.modifiers == KeyModifiers::CONTROL {
		// Always allow the program to be closed via Ctrl-C
		eng.running = false;
	}
	if eng.show_main_menu {
		// TODO: find a way to generalize this to allow the same logic for inventory selection
		// use the meta mappings
		match key_event.code {
			// Toggle main menu/pause on Esc or Q
			KeyCode::Esc | KeyCode::Char('Q') => {
				eng.pause_toggle();
				eng.main_menu_toggle();
			}
			// Scroll the menu
			KeyCode::Char('j') | KeyCode::Down => {
				eng.main_menu.next();
			}
			KeyCode::Char('k') | KeyCode::Up => {
				eng.main_menu.prev();
			}
			KeyCode::Left => { eng.main_menu.deselect(); }
			// Confirm selection
			KeyCode::Enter => {
				let choice = eng.main_menu.state.selected();
				if choice.is_some() {
					let choice_val = choice.unwrap_or_default();
					// trying to extract/convert to the menu item is a pain and only gets a String
					// consider setting up an enum for the main menu options
					let choice_text = match choice_val {
						1 => MainMenuItems::NEWGAME,
						2 => MainMenuItems::LOADGAME,
						3 => MainMenuItems::SAVEGAME,
						4 => MainMenuItems::QUIT,
						_ => MainMenuItems::NULL,
					};
					eprintln!("Selection: {}", choice_text);// DEBUG:
					match choice_text {
						MainMenuItems::NEWGAME => {
							eprintln!("NEWGAME called");
						}
						MainMenuItems::LOADGAME => {
							eprintln!("LOADGAME called");
						}
						MainMenuItems::SAVEGAME => {
							eprintln!("SAVEGAME called");
						}
						MainMenuItems::QUIT => {
							eng.running = false;
						}
						MainMenuItems::NULL => { }
					}
					// Then clear off the screen and return to the game
					eng.pause_toggle();
					eng.main_menu_toggle();
					eng.main_menu.deselect();
					return Ok(())
				}
			}
			// Else, do nothing
			_ => {}
		}
	} else { // this should be the 'default' game interaction mode
		// use the literal mappings
		match key_event.code {
			// Pause key
			KeyCode::Char('p') => {
				eng.pause_toggle();
			}
			// Pause and show main menu on `ESC` or `Q`
			KeyCode::Esc | KeyCode::Char('Q') => {
				eng.pause_toggle();
				eng.main_menu_toggle();
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
	}
	Ok(())
}

// EOF
