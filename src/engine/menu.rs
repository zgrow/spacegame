// engine/menu.rs
// Rebuilt from previous method from pattern found at shuoli84/tui-menu

use std::borrow::Cow;
use std::marker::PhantomData;
use std::cmp::Ordering;
use ratatui::{
	buffer::Buffer,
	layout::Rect,
	style::{Color, Style},
	text::Span,
	widgets::{
		Block,
		Clear,
		StatefulWidget,
		Widget
	},
};
use crate::engine::*;

/// Describes the various types of menus (actually MenuStates, see the GameEngine) we might wish to use
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MenuType {
	#[default]
	None,
	Main,
	Entity,
	Action,
	Context,
}
/// Describes the set of Events that the Menu widget may produce
#[derive(Clone, Copy, Debug)]
pub enum MenuEvent<T> {
	/// The selected menu item, with its attached data
	Selected(T),
}
/// Helper bucket that is used to carry potential action context info through the menu system for later dispatch
pub struct GameEventPartial {
	pub action: ActionType,
	pub subject: Entity,
	pub object: Entity,
}
impl GameEventPartial {
	pub fn new(new_action: Option<ActionType>, new_subject: Option<Entity>, new_object: Option<Entity>) -> GameEventPartial {
		GameEventPartial {
			action: match new_action {
				None => ActionType::NoAction,
				Some(action) => action,
			},
			subject: match new_subject {
				None => Entity::PLACEHOLDER,
				Some(enty) => enty,
			},
			object: match new_object {
				None => Entity::PLACEHOLDER,
				Some(enty) => enty,
			},
		}
	}
	/// Returns true if it has a populated ActionType and the non-placeholder entity references to fulfill it;
	/// does NOT guarantee that the entities refer to valid objects!
	pub fn is_complete(&self) -> bool {
		match self.action {
			ActionType::MoveTo(_)
			| ActionType::Inventory
			| ActionType::KillItem => {
				self.subject != Entity::PLACEHOLDER
			},
			ActionType::Examine
			| ActionType::MoveItem
			| ActionType::DropItem
			| ActionType::UseItem
			| ActionType::OpenItem
			| ActionType::CloseItem
			| ActionType::LockItem
			| ActionType::UnlockItem
			=> {
				self.subject != Entity::PLACEHOLDER && self.object != Entity::PLACEHOLDER
			},
			ActionType::NoAction => {
				false
			}
		}
	}
}
impl Default for GameEventPartial {
	fn default() -> GameEventPartial {
		GameEventPartial {
			action: ActionType::NoAction,
			subject: Entity::PLACEHOLDER,
			object: Entity::PLACEHOLDER,
		}
	}
}
/// Holds the menu's current state and the events it generated
pub struct MenuState<T> {
	menu_tree: MenuItem<T>,
	events: Vec<MenuEvent<T>>,
	pub width: usize,
	pub target: Option<Position>,
}
impl<T: Clone> MenuState<T> {
	/// Allows creation of the menu with items
	/// Example:
	/// ...
	/// let state = MenuState::<&'static str>::new(vec![
	///   MenuItem::item("Foo", "label_foo"),
	///   MenuItem::group("Group", vec![
	///     MenuItem::item("Bar", "label_bar"),
	///     MenuItem::item("Qaz", "label_qaz"),
	///   ])
	/// ]);
	/// ...
	pub fn new(items: Vec<MenuItem<T>>) -> Self {
		let mut max_width = 0;
		for entry in items.iter() {
			if max_width < entry.width {
				max_width = entry.width;
			}
		}
		Self {
			menu_tree: MenuItem {
				name: "root".into(),
				data: None,
				target: None,
				width: max_width,
				children: items,
				is_highlighted: true, // Required to keep highlighting logic more consistent
			},
			events: Default::default(),
			width: max_width,
			target: None,
		}
	}
	/// Proceed with execution of the selected menu item
	pub fn activate(&mut self) {
		self.menu_tree.highlight_next();
	}
	/// Move the menu cursor up
	//  NOTE: The movement logic for up/down prefers intuitive over logical, so is not always consistent:
	//  CASE 1: Selection moves from sub Item of Group up to Group selection
	//    Group 1    Group 2    Group 3
	//               > Item 1
	//                 Item 2
	//  -> Pressing Up here executes pop(), which closes Group 2
	//  CASE 2: Selection moves from sub Item to sub item
	//    Group 1    Group 2    Group 3
	//                 Item 1
	//               > Item 2
	//  -> Pressing Up here executes move_prev()
	//  CASE 3: Selection moves from 1st sub-sub Item to 2nd sub-item
	//    Group 1    Group 2    Group 3
	//                 Item 1
	//               > Item 2 - > Sub-item 1
	//                            Sub-Item 2
	//  -> Pressing Up here does nothing(!), as it is technically at the 'top' of a menu: press Left instead
	pub fn up(&mut self) {
		/* OLD METHOD
		match self.active_depth() {
			0 | 1 => { /* do nothing */ }
			2     => match self.menu_tree.highlight_child().and_then(|child| child.highlight_child_index()) {
				Some(index) if index == 0 => { self.pop(); }
				_                         => { self.prev(); }
			}
			_     => { self.prev(); }
		}*/
		self.prev();
	}
	/// Move the menu cursor down
	//  NOTE: As with up(), the movement logic is not as consistent/logical as expected:
	//  CASE 1: Selection moves from Group down to Item 1
	//  -> Down executes enter(), which moves the cursor into the Group's subgroup
	//  CASE 2: Selection tries to move down from last Item in subgroup
	//  -> Nothing occurs
	//  CASE 3: Selection moves from Item 1 to Item 2
	//  -> Down highlights the next Item
	pub fn down(&mut self) {
		/* OLD METHOD
		if self.active_depth() == 1 {
			self.push();
		} else {
			self.next();
		}*/
		self.next();
	}
	/// Move the menu cursor left
	//  Moving from Group to Group is normal
	//  Moving from menu to Group closes the menu, then moves the cursor
	//  Moving from submenu to menu closes the submenu
	pub fn left(&mut self) {
		// OLD METHOD
		if self.active_depth() == 1 {
			self.prev();
		} else if self.active_depth() == 2 {
		// ERROR: pressing left() twice at the root menu causes the cursor to become unusable:
		//        the cursor still moves U/D as if selecting, but nothing activates on Enter and submenus cannot be moved into
		//if self.active_depth() == 2 {
			self.pop();
			self.prev();
		} else {
			self.pop();
		}
	}
	/// Move the menu cursor right
	//  Moving from Group to Group is normal
	//  Moving from menu to Group executes pop() on the Sub-item, then highlights Group 3
	//  Expanding a submenu executes push() on the submenu's item; preserves highlighting
	pub fn right(&mut self) {
		/* OLD METHOD
		if self.active_depth() == 1 {
			self.next();
		} else if self.active_depth() == 2 {*/
		if self.active_depth() == 2 {
			if self.push().is_none() {
				// "Special handling, make menu navigation more productive"
				self.pop();
				self.next();
			}
		} else {
			self.push();
		}
	}
	/// Highlight the previous Item in the current group
	/// If the first Item is selected, does nothing.
	fn prev(&mut self) {
		if let Some(item) = self.menu_tree.highlight_last_but_one() {
			self.target = item.highlight_prev();
		} else {
			self.target = self.menu_tree.highlight_prev();
		}
	}
	/// Highlight the next Item in the current Group
	/// If the last Item is selected, then does nothing.
	fn next(&mut self) {
		if let Some(item) = self.menu_tree.highlight_last_but_one() {
			self.target = item.highlight_next();
		} else {
			self.target = self.menu_tree.highlight_next();
		}
	}
	/// Returns the active depth, ie how many submenus have been expanded
	fn active_depth(&self) -> usize {
		let mut item = self.menu_tree.highlight_child();
		let mut depth = 0;
		while let Some(inner_item) = item {
			depth += 1;
			item = inner_item.highlight_child();
		}
		depth
	}
	/// Selects the currently-highlighted item, if it has children, then executes push()
	pub fn select(&mut self) {
		if let Some(item) = self.menu_tree.highlighted_mut() {
			if !item.children.is_empty() {
				self.push();
			} else if let Some(ref data) = item.data {
				self.events.push(MenuEvent::Selected(data.clone()));
			}
		}
	}
	/// Opens a submenu, if applicable
	/// Returns Some if it found a submenu to enter, None otherwise
	pub fn push(&mut self) -> Option<Position> {
		self.target = None;
		self.menu_tree.highlighted_mut()?.highlight_first_child()
	}
	/// Closes the current submenu and moves up a level
	pub fn pop(&mut self) {
		if let Some(item) = self.menu_tree.highlighted_mut() {
			item.clear_highlight();
		}
	}
	/// Clears all highlights
	pub fn reset(&mut self) {
		self.menu_tree.children.iter_mut().for_each(|c| c.clear_highlight());
		self.target = None;
	}
	/// Cleans out the event queue, helps prevent lag: consider executing this on every frame
	pub fn drain_events(&mut self) -> impl Iterator<Item = MenuEvent<T>> {
		std::mem::take(&mut self.events).into_iter()
	}
	/// Returns the reference of the currently selected Item
	pub fn highlight(&mut self) -> Option<&MenuItem<T>> {
		self.menu_tree.highlighted()
	}
}
/// Describes a single entry in a Menu
pub struct MenuItem<T> {
	name: Cow<'static, str>,
	pub data: Option<T>,
	pub target: Option<Position>,
	pub width: usize, /// Set this to the length of the MenuItem's name, so that the menu render logic knows how much room to allot
	children: Vec<MenuItem<T>>,
	is_highlighted: bool,
}
impl<T> MenuItem<T> {
	/// Creates a single menu entry with a data entry, no submenu group
	pub fn item(name: impl Into<Cow<'static, str>>, data: T, new_target: Option<Position>) -> Self {
		let new_name: Cow<'static, str> = name.into();
		Self {
			name: new_name.clone(),
			data: Some(data),
			target: new_target,
			width: new_name.len(),
			is_highlighted: false,
			children: vec![],
		}
	}
	/// Creates a submenu group, no data
	pub fn group(name: impl Into<Cow<'static, str>>, children: Vec<Self>) -> Self {
		let new_name: Cow<'static, str> = name.into();
		Self {
			name: new_name.clone(),
			data: None,
			target: None,
			width: new_name.len(),
			is_highlighted: false,
			children,
		}
	}
	pub fn is_group(&self) -> bool {
		!self.children.is_empty()
	}
	fn name(&self) -> &str {
		&self.name
	}
	fn highlight_first_child(&mut self) -> Option<Position> {
		if !self.children.is_empty() {
			let mut posn = None;
			if let Some(thing) = self.children.get_mut(0) {
				posn = thing.set_highlight();
			}
			return posn;
		}
		None
	}
	fn highlight_prev(&mut self) -> Option<Position> {
		// If no child is selected, then
		let Some(index) = self.highlight_child_index() else {
			return self.highlight_first_child();
		};
		let index_to_highlight = if index > 0 {
			index - 1
		} else {
			0
		};
		self.children[index].clear_highlight();
		self.children[index_to_highlight].set_highlight()
	}
	fn highlight_next(&mut self) -> Option<Position> {
		let Some(index) = self.highlight_child_index() else {
			return self.highlight_first_child();
		};
		// If no child is selected, then
		let index_to_highlight = (index + 1).min(self.children.len() - 1);
		self.children[index].clear_highlight();
		self.children[index_to_highlight].set_highlight()
	}
	fn highlight_child_index(&self) -> Option<usize> {
		for (index, child) in self.children.iter().enumerate() {
			if child.is_highlighted {
				return Some(index);
			}
		}
		None
	}
	fn highlight_child(&self) -> Option<&Self> {
		self.children.iter().find(|c| c.is_highlighted)
	}
	fn highlight_child_mut(&mut self) -> Option<&mut Self> {
		self.children.iter_mut().find(|c| c.is_highlighted)
	}
	fn clear_highlight(&mut self) {
		self.is_highlighted = false;
		for child in self.children.iter_mut() {
			child.clear_highlight();
		}
	}
	fn set_highlight(&mut self) -> Option<Position> {
		self.is_highlighted = true;
		self.target
	}
	fn highlighted(&mut self) -> Option<&Self> {
		if !self.is_highlighted {
			return None;
		}
		let mut highlighted_item = self;
		while highlighted_item.highlight_child_mut().is_some() {
			highlighted_item = highlighted_item.highlight_child_mut().unwrap();
		}
		Some(highlighted_item)
	}
	fn highlighted_mut(&mut self) -> Option<&mut Self> {
		if !self.is_highlighted {
			return None;
		}
		let mut highlighted_item = self;
		while highlighted_item.highlight_child_mut().is_some() {
			highlighted_item = highlighted_item.highlight_child_mut().unwrap();
		}
		Some(highlighted_item)
	}
	fn highlight_last_but_one(&mut self) -> Option<&mut Self> {
		// If self or a child is not highlighted, return None
		if !self.is_highlighted || self.highlight_child_mut().is_none() {
			return None;
		}
		let mut penultimate = self;
		while penultimate.highlight_child_mut().and_then(|x| x.highlight_child_mut()).is_some() {
			penultimate = penultimate.highlight_child_mut().unwrap();
		}
		Some(penultimate)
	}
}
impl<T> PartialEq for MenuItem<T> {
	fn eq(&self, other: &Self) -> bool {
		self.name == other.name
	}
}
impl<T> PartialOrd for MenuItem<T> {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.name.cmp(&other.name))
	}
}
/// Describes the container for several MenuItems
pub struct Menu<'a, T> {
	block: Option<Block<'a>>,
	default_style: Style,
	highlight_style: Style,
	drop_width: u16,
	drop_style: Style,
	shadow_style: Style,
	_priv: PhantomData<T>,
}
impl<T> Default for Menu<'_, T> {
	fn default() -> Self {
		Self::new()
	}
}
impl<'a, T> Menu<'a, T> {
	pub fn new() -> Self {
		Self {
			block: None,
			default_style: Style::default().fg(Color::Black).bg(Color::Gray),
			highlight_style: Style::default().fg(Color::Black).bg(Color::White),
			drop_width: 20,
			drop_style: Style::default().fg(Color::Black).bg(Color::Gray),
			shadow_style: Style::default().fg(Color::Red).bg(Color::DarkGray),
			_priv: Default::default(),
		}
	}
	pub fn block(mut self, block: Block<'a>) -> Self {
		self.block = Some(block);
		self
	}
	pub fn default_style(mut self, style: Style) -> Self {
		self.default_style = style;
		self
	}
	pub fn highlight_style(mut self, style: Style) -> Self {
		self.highlight_style = style;
		self
	}
	pub fn dropdown_width(mut self, width: u16) -> Self {
		self.drop_width = width;
		self
	}
	pub fn dropdown_style(mut self, style: Style) -> Self {
		self.drop_style = style;
		self
	}
	fn render_drop_down(&self, x: u16, y: u16, group: &[MenuItem<T>], buf: &mut Buffer, _depth: usize) {
		debug!("* Rendering drop down..."); // DEBUG: announce render_drop_down
		let area = Rect::new(x, y, self.drop_width, group.len() as u16);
		self.render_shadow(area, buf);
		Clear.render(area, buf);
		buf.set_style(area, self.drop_style);
		for (index, item) in group.iter().enumerate() {
			let item_y = y + index as u16;
			let is_active = item.is_highlighted;
			buf.set_span(
				x,
				item_y,
				&Span::styled(
					item.name(),
					if is_active {
						self.highlight_style
					} else {
						self.default_style
					},
				),
				self.drop_width,
			);
			if is_active && !item.children.is_empty() {
				self.render_drop_down(
					x + self.drop_width,
					item_y,
					&item.children,
					buf,
					// INFO: the line below was part of the original example, but clippy says (correctly!) that this line is only used
					//       in recursion and *nothing else*! Therefore, before removing it entirely, it is critical to ascertain why
					//       it's even here in the first place...
					_depth + 1
				);
			}
		}
	}
	/// Draws the drop-shadow underneath a menu, given the area it will occupy
	/// Note that this does NOT clear the menu's area after drawing into it; the caller must do so before drawing the menu
	/// This helps ensure that nothing is removed that shouldn't be
	fn render_shadow(&self, area: Rect, buf: &mut Buffer) {
		let shadow = Rect::new(area.x + 1, area.y + 1, area.width, area.height); // Calculate the shadow's dims
		Clear.render(shadow, buf); // Clear the shadow's draw area
		buf.set_style(shadow, self.shadow_style); // Assign the style we'll use
		let empty_line = " ".repeat(shadow.width.into()); // Create a placeholder line of the correct length
		for line in 0..area.height {
			buf.set_span(shadow.x, shadow.y + line,
									 &Span::styled(empty_line.clone(), Style::default()), 1); // Write the line to the screen
		}
	}
}
impl<T> StatefulWidget for Menu<'_, T> {
	type State = MenuState<T>;
	fn render(mut self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
		// Draw the border, if it exists
		let area = match self.block.take() {
			Some(b) => {
				let inner_area = b.inner(area);
				b.render(area, buf);
				inner_area
			}
			None => area,
		};
		// Render the title
		self.render_shadow(area, buf);
		self.render_drop_down(area.x, area.y, &state.menu_tree.children, buf, 1);
	}
}

// EOF
