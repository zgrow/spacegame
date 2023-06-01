// app/event.rs
// Contains the defns for my Bevy event types

use bevy::prelude::*;
use crate::components::*;
use std::fmt;

//  *** GAME EVENTS
/// Describes a general game event, can include a GameEventContext
#[derive(Resource, Default)]
pub struct GameEvent {
	pub etype: GameEventType,
	pub context: Option<GameEventContext>,
}
impl GameEvent {
	pub fn new(new_type: GameEventType, new_context: Option<GameEventContext>) -> GameEvent {
		GameEvent {
			etype: new_type,
			context: new_context,
		}
	}
}
/// Provides the descriptors for GameEvents
/// Unless otherwise noted, any relevant event info will be included as a GameEventContext
/// TODO: optimize this to break up the events into different classes/groups so that the event
/// readers in the various subsystems only have to worry about their specific class of events
#[derive(Copy, Clone, Eq, PartialEq, Default)]
pub enum GameEventType {
	#[default]
	NullEvent,
	PauseToggle, // specifically causes a mode switch between Running <-> Paused
	ModeSwitch(EngineMode), // switches the engine to the specified mode
	PlayerMove(Direction),
	ActorOpen,
	ActorClose,
	ActorLock,
	ActorUnlock,
	ItemUse,
	ItemMove,
	ItemDrop,
	ItemKILL,
	DoorOpen,
	DoorClose,
}
impl fmt::Display for GameEventType {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let output = match self {
			GameEventType::NullEvent => { "etype::NullEvent" }
			GameEventType::PauseToggle => { "etype::PauseToggle" }
			GameEventType::ModeSwitch(_) => { "etype::ModeSwitch" }
			GameEventType::PlayerMove(_) => { "etype::PlayerMove" }
			GameEventType::ActorOpen => { "etype::ActorOpen" }
			GameEventType::ActorClose => { "etype::ActorClose" }
			GameEventType::ActorLock => { "etype::ActorLock" }
			GameEventType::ActorUnlock => { "etype::ActorUnlock" }
			GameEventType::ItemUse => { "etype::ItemUse" }
			GameEventType::ItemMove => { "etype::ItemMove" }
			GameEventType::ItemDrop => { "etype::ItemDrop" }
			GameEventType::ItemKILL => { "etype::ItemKILL" }
			GameEventType::DoorOpen => { "etype::DoorOpen" }
			GameEventType::DoorClose => { "etype::DoorClose" }
		};
		write!(f, "{}", output)
	}
}
/// Friendly bucket for holding contextual information about game actions
/// Note that this expresses a 1:1 relation: this preserves the atomic nature of the event
/// If an event occurs with multiple objects, then that event should be broken into multiple
#[derive(Resource, Copy, Clone, Eq, PartialEq)]
pub struct GameEventContext {
	pub subject: Entity, // the entity performing the action; by defn, only one
	pub object: Entity, // the entity upon which the subject will perform the action
}
impl GameEventContext {
	/// Returns true if either of the context elements are set to the Placeholder
	pub fn is_invalid(&self) -> bool {
		if self.subject == Entity::PLACEHOLDER
		&& self.object == Entity::PLACEHOLDER { return true; }
		false
	}
}

//  *** PLANQ EVENTS
/// Describes a PLANQ-specific event, ie an event connected to its logic
#[derive(Resource, Copy, Clone, Eq, PartialEq, Reflect, FromReflect, Default)]
pub struct PlanqEvent {
	pub etype: PlanqEventType,
}
impl PlanqEvent {
	pub fn new(new_type: PlanqEventType) -> PlanqEvent {
		PlanqEvent {
			etype: new_type,
		}
	}
}
/// Defines the set of control and input events that the Planq needs to handle
#[derive(Copy, Clone, Eq, PartialEq, Debug, Resource, Reflect, FromReflect, Default)]
pub enum PlanqEventType {
	#[default]
	NullEvent,
	Startup,
	BootStage(u32),
	Shutdown,
	Reboot,
	GoIdle,
	CliOpen,
	CliClose,
	InventoryUse,
	InventoryDrop,
}

// EOF
