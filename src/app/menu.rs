// menu.rs
// Provides a Menu widget to ratatui

use ratatui::widgets::{StatefulWidget, ListState};
use ratatui::layout::Rect;
use ratatui::buffer::Buffer;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use std::fmt;

/// Provides the full list of options for the main menu
/// The Ord/PartialOrd traits add implicit indexing to this enum
/// e.g. NULL == 0, NEWGAME == 1; NULL < NEWGAME == true
#[derive(Debug, EnumIter, PartialEq, Eq, PartialOrd, Ord)]
pub enum MainMenuItems {
	NULL,
	NEWGAME,
	LOADGAME,
	SAVEGAME,
	QUIT
}
impl MainMenuItems {
	pub fn to_list() -> Vec<MainMenuItems> {
		let mut list = Vec::new();
		for val in MainMenuItems::iter() {
			if val == MainMenuItems::NULL { continue; } // Don't add NULL to lists
			list.push(val);
		}
		return list;
	}
}
impl fmt::Display for MainMenuItems {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			MainMenuItems::NULL => write!(f, "NULL"),
			MainMenuItems::NEWGAME => write!(f, "New Game"),
			MainMenuItems::LOADGAME => write!(f, "Load Game"),
			MainMenuItems::SAVEGAME => write!(f, "Save Game"),
			MainMenuItems::QUIT => write!(f, "Quit"),
		}
	}
}

#[derive(Clone)]
pub struct MenuSelector<T> {
	pub list: Vec<T>, // the state as it relates to my application
	pub state: ListState, // the UI state, incl index of selection and its offset for draw calls
}
impl<Entity> MenuSelector<Entity> {

}
impl<MainMenuItems> MenuSelector<MainMenuItems> {
	pub fn with_items(items: Vec<MainMenuItems>) -> MenuSelector<MainMenuItems> {
		MenuSelector {
			list: items,
			state: ListState::default(),
		}
	}
	pub fn next(&mut self) {
		let index = match self.state.selected() {
			Some(index) => {
				if index >= (self.list.len() - 1) {
					0
				} else {
					index + 1
				}
			}
			None => 0,
		};
		self.state.select(Some(index));
	}
	pub fn prev(&mut self) {
		let index = match self.state.selected() {
			Some(index) => {
				if index == 0 {
					self.list.len() - 1
				} else {
					index - 1
				}
			}
			None => 0,
		};
		self.state.select(Some(index));
	}
	pub fn deselect(&mut self) {
		self.state.select(None);
	}
}
impl<T> StatefulWidget for MenuSelector<T> {
	type State = ListState;
	fn render(self, _area: Rect, _buf: &mut Buffer, _state: &mut Self::State) { }
}

// EOF
