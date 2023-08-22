// lib.rs
// July 12 2023

// Provides the GameEngine with an internal Bevy instance and related methods
pub mod engine;

// Collection of smaller Components for Bevy that aren't directly associated with a particular type
pub mod components;

// Collection of Systems for Bevy that aren't directly associated with a particular type
pub mod sys;

// Provides the prototypes and logic for the GameWorld world model object
pub mod map;

// Provides the abstraction onto the game world for rendering onto a display
pub mod camera;

// Provides the map builder
pub mod mason;

// Provides the item builder
pub mod artisan;

// EOF
