// engine/event.rs
// Provides the in-game events and related logics

// *** EXTERNAL LIBS
use bevy::prelude::*;
use bevy::ecs::entity::*;
use strum_macros::AsRefStr;
use std::fmt::{Display, Formatter, Result};

// *** INTERNAL LIBS
use crate::components::Direction;
use crate::engine::*;

//  *** GAME EVENTS
/// Describes a general game event, can include a GameEventContext
#[derive(Event, Clone, Copy, Debug, Default, Reflect)]
pub struct GameEvent {
	pub etype: GameEventType,
	pub context: Option<GameEventContext>,
}
impl GameEvent {
	pub fn new(new_type: GameEventType, new_subject: Option<Entity>, new_object: Option<Entity>) -> GameEvent {
		let new_context = GameEventContext {
			subject: match new_subject {
				None => { Entity::PLACEHOLDER }
				Some(enty) => { enty }
			},
			object: match new_object {
				None => { Entity::PLACEHOLDER }
				Some(enty) => { enty }
			},
		};
		GameEvent {
			etype: new_type,
			context: if new_context.is_blank() { None } else { Some(new_context) },
		}
	}
	pub fn is_valid(&self) -> bool {
		match self.etype {
			GameEventType::NullEvent => { false }
			GameEventType::PauseToggle => { true }
			GameEventType::ModeSwitch(_) => { true }
			GameEventType::ActorAction(action) | GameEventType::PlayerAction(action) => {
				if let Some(context) = self.context { // Did the action have a context attached?
					match action {
						// Requires only a subject
						ActionType::MoveTo(_)
						=> {
							if let Some(context) = self.context {
								context.subject != Entity::PLACEHOLDER
							} else { false }
						}
						// Requires both a subject and an object
						ActionType::Examine
						| ActionType::UseItem
						| ActionType::MoveItem
						| ActionType::DropItem
						| ActionType::KillItem
						| ActionType::OpenItem
						| ActionType::CloseItem
						=> {
							context.subject != Entity::PLACEHOLDER && context.object != Entity::PLACEHOLDER
						}
						_ => {
							eprintln!("* ActionType::{} had a context when validation was attempted", action); // DEBUG: report an event validation error
							false
						} // If it had a context but didn't match one of the types above, it's probably malformed
					}
				} else { // Does not require any context
					if action == ActionType::Inventory { return true; }
					false
				}
			}
		}
	}
}
impl Display for GameEvent {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.etype)
	}
}
/// Allows comparison of two variant enums without regard to their type, ie
///   `ModeSwitch(Paused) == ModeSwitch(Running)`
/// should return TRUE where Rust would return FALSE
pub fn same_enum_variant<T>(a: &T, b: &T) -> bool {
	std::mem::discriminant(a) == std::mem::discriminant(b)
}
/// Provides the descriptors for GameEvents
/// Unless otherwise noted, any relevant event info will be included as a GameEventContext
#[derive(AsRefStr, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub enum GameEventType {
	#[default]
	NullEvent,
	PauseToggle, // specifically causes a mode switch between Running <-> Paused
	ModeSwitch(EngineMode), // switches the engine to the specified mode
	PlayerAction(ActionType),
	ActorAction(ActionType),
}
impl Display for GameEventType {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result {
		let output = match self {
			GameEventType::NullEvent             => { "NullEvent".to_string() }
			GameEventType::PauseToggle           => { "PauseToggle".to_string() }
			GameEventType::ModeSwitch(mode)      => { format!("ModeSwitch({:?})", mode) }
			GameEventType::PlayerAction(action)  => { format!("{}", action) }
			GameEventType::ActorAction(action)   => { format!("{}", action) }
		};
		let prim = output.as_str();
		write!(f, "{}", prim)
	}
}
/// Describes the set of actions that may be performed by any of the entities in the game
#[derive(AsRefStr, Component, Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
pub enum ActionType {
	#[default]          // TARGET
	NoAction,           // not associated with any Components, by definition
	Examine,            // Description
	MoveTo(Direction),  // Mobile
	Inventory,          // [Player: indicates that they've opened the inventory to use an item in it]
	MoveItem,           // Portable
	DropItem,           // Portable
	UseItem,            // (none, probably Operable? when set up)
	KillItem,           // *system action only*, not associated with any Components
	OpenItem,           // Openable
	CloseItem,          // Openable
	//LockItem,         // Lockable
	//UnlockItem,       // Lockable
}
//serialize_trait_object!(ActionType);
impl Display for ActionType {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		// WARN: Because of how Rust constructs the temporaries that it uses while handling match cases,
		//       trying to use an "if let output = match self { ... }" statement here causes massive issues
		//       because Rust wants to keep the temporary until *after* the fucking variable gets alllll put together
		//         which means the temporary effectively has to last as long as the variable being assigned to,
		//           which requires assigning the temporary to a permanent using "let",
		//             which can't be fucking done in a match arm because the temp value is the fucking enum variant value,
		//             and Rust provides no fucking way to use a "let" statement in that position of the syntax,
		//               so we just have to accept that it's going to be shitty ( > m <)
		let output = match self {
			ActionType::NoAction     => { "NoAction".to_string() }
			ActionType::Examine      => { "Examine".to_string() }
			ActionType::MoveTo(dir)  => { format!("MoveTo({})", dir) }
			ActionType::Inventory    => { "Inventory".to_string() }
			ActionType::MoveItem     => { "Move".to_string() }
			ActionType::DropItem     => { "Drop".to_string() }
			ActionType::UseItem      => { "Use".to_string() }
			ActionType::KillItem     => { "KillItem".to_string() }
			ActionType::OpenItem     => { "Open".to_string() }
			ActionType::CloseItem    => { "Close".to_string() }
			/*
			ActionType::LockItem     => { "atype::LockItem" }
			ActionType::UnlockItem   => { "atype::UnlockItem" }
			*/
			//_ => { "atype::UNKNOWN" }
		};
		// Trying to write the output var directly causes major borrow issues
		// Using the output var as an interstitial allows us to use format! to build the string dynamically
		let prim = output.as_str();
		write!(f, "{}", prim)
	}
}
impl From<ActionType> for Cow<'_, str> {
	fn from(a_type: ActionType) -> Self {
		/*
		let value = match a_type {
			ActionType::NoAction     => { "_NoAction".to_string() }
			ActionType::Examine      => { "Examine".to_string() }
			ActionType::MoveTo(dir)  => { format!("_MoveTo({})", dir) }
			ActionType::Inventory    => { "Inventory".to_string() }
			ActionType::MoveItem     => { "Move".to_string() }
			ActionType::DropItem     => { "Drop".to_string() }
			ActionType::UseItem      => { "Use".to_string() }
			ActionType::KillItem     => { "_KillItem".to_string() }
			ActionType::OpenItem     => { "Open".to_string() }
			ActionType::CloseItem    => { "Close".to_string() }
		};*/
		let pack = Cow::Owned(format!("{}", a_type).clone());
		pack
	}
}
/// Friendly bucket for holding contextual information about game actions
/// Note that this expresses a 1:1 relation: this preserves the atomic nature of the event
/// If an event occurs with multiple objects, then that event should be broken into multiple
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Resource)]
pub struct GameEventContext {
	pub subject: Entity, // the entity performing the action; by defn, only one
	pub object: Entity, // the entity upon which the subject will perform the action
}
impl GameEventContext {
	pub fn new(actor: Entity, target: Entity) -> GameEventContext {
		GameEventContext {
			subject: actor,
			object: target,
		}
	}
	/// Returns true if either of the context elements are set to the Placeholder
	pub fn is_partial(&self) -> bool {
		self.subject == Entity::PLACEHOLDER || self.object == Entity::PLACEHOLDER
	}
	/// Returns true IFF both of the context elements are set to the Placeholder
	pub fn is_blank(&self) -> bool {
		self.subject == Entity::PLACEHOLDER && self.object == Entity::PLACEHOLDER
	}
}
impl Default for GameEventContext {
	fn default() -> GameEventContext {
		GameEventContext {
			subject: Entity::PLACEHOLDER,
			object: Entity::PLACEHOLDER,
		}
	}
}
impl MapEntities for GameEventContext { // Maintain Entity references wrt bevy_save
	fn map_entities(&mut self, entity_mapper: &mut EntityMapper) {
		self.subject = entity_mapper.get_or_reserve(self.subject);
		self.object = entity_mapper.get_or_reserve(self.object);
	}
}

// EOF
