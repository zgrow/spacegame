// engine/camera.rs
// Provides implementation for the CameraView component, including refresh/update logic

#![allow(clippy::type_complexity)]

// *** EXTERNAL LIBS
use bevy::prelude::{
	Component,
	Entity,
	Reflect,
	ReflectComponent,
	ReflectResource,
	Res,
	ResMut,
	Resource,
	Query,
	With,
	Without,
};
use bracket_geometry::prelude::*;
use ratatui::style::Color;
use ratatui::buffer::Cell;
use ratatui::style::Modifier;
use simplelog::*;

// *** INTERNAL LIBS
use crate::components::*;
use crate::worldmap::*;

/// Represents a 'flattened' view of the Map's layers, with all entities and effects painted in,
/// such that it can be read by the Viewport object when it comes time to render the view
/// Provides an abstraction to the Viewport widget with hooks into Bevy's systems for updates
#[derive(Component, Resource, Clone, Debug, Default, Reflect)]
#[reflect(Component, Resource)]
pub struct CameraView {
	pub output: Vec<ScreenCell>,
	pub width: i32,
	pub height: i32,
	pub reticle: Position,
	pub reticle_glyphs: String,
	pub terrain: Vec<ScreenCell>, // The map of all the 'base' tiles, provides the 'fallback' minimum visual for rendering
	pub scenery: Vec<ScreenCell>, // The map of all the scenery and furniture objects
	pub actors: Vec<ScreenCell>, // The map of all the Actor glyphs
	pub blinken: Vec<ScreenCell>, // The map of all the 'scenery' effects: glowing screens and other cycling animations
	pub vfx: Vec<ScreenCell>, // The map of the special effects feedback visuals - short-term and incidental
}
impl CameraView {
	pub fn new(new_width: i32, new_height: i32) -> Self {
		Self {
			output: vec![ScreenCell::default(); (new_width * new_height) as usize],
			width: new_width,
			height: new_height,
			reticle: Position::INVALID,
			reticle_glyphs: "⌟⌞⌝⌜".to_string(), // Corner frame
			terrain: vec![ScreenCell::default(); (new_width * new_height) as usize],
			scenery: vec![ScreenCell::default(); (new_width * new_height) as usize],
			actors: vec![ScreenCell::default(); (new_width * new_height) as usize],
			blinken: vec![ScreenCell::default(); (new_width * new_height) as usize],
			vfx: vec![ScreenCell::default(); (new_width * new_height) as usize],
		}
		// Other options for reticles might include: (not all tested)
		// The reticle glyph order is UL, UR, DL, DR
		//	reticle_glyphs: "JL7F".to_string(), // 1337code fallback, guaranteed to work everywhere lol
		//	reticle_glyphs: "\\//\\".to_string(), // Slashes in the corners
		//	reticle_glyphs: "▗▖▝▘".to_string(), // Four small corner boxes
		//	reticle_glyphs: "▚▞▞▚".to_string(), // Blocky slashes
		//	reticle_glyphs: "◞◟◝◜".to_string(), // Curved corners (might have unicode issues)
		//	reticle_glyphs: "◿◺◹◸".to_string(), // Triangular corners
		//	reticle_glyphs: "⌟⌞⌝⌜".to_string(), // Corner frame
		//	reticle_glyphs: "⌌⌍⌎⌏".to_string(), // Square frame
		//	reticle_glyphs: "|\/".to_string(), // need to impl a 3-point reticle in the logic below
	}
	pub fn set_dims(&mut self, new_width: i32, new_height: i32) {
		// TODO: include a sanity check here that actually examines the dims prior to resize
		// if the resize is required, then probably safest to wipe the whole thing...
		// either way, make sure that the CameraView gets an update before next render call
		self.width = new_width;
		self.height = new_height;
		let new_size = (self.width * self.height) as usize;
		if self.output.len() != new_size {
			self.output = vec![ScreenCell::default(); new_size];
		}
	}
	/*
	pub fn iterate_countdown(&mut self) {
		for effect in self.fx.iter_mut() {
			if effect.countdown > 0 { effect.countdown -= 1; }
		}
	}
	*/
}
/// Compatibility type for better integration with ratatui; converts directly to a ratatui::Buffer::Cell
#[derive(Component, Resource, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[reflect(Component, Resource)]
pub struct ScreenCell {
	pub glyph: String,
	pub fg: u8,
	pub bg: u8,
	pub modifier: u16,
	// The Cell::underline_color and Cell::skip fields are not needed
}
impl From<ScreenCell> for Cell { // Used for converting my custom ScreenCell objects into ratatui::Cells for processing
	fn from(input: ScreenCell) -> Self {
		Cell {
			symbol: input.glyph.clone(),
			fg: Color::Indexed(input.fg),
			bg: Color::Indexed(input.bg),
			underline_color: Color::LightMagenta, // DEBUG: This is intentionally set to a trash color as I do not plan to make use of it at this time
			modifier: Modifier::from_bits(input.modifier).unwrap_or(Modifier::empty()),
		}
	}
}
impl From<Tile> for ScreenCell { // Used for converting my custom Tile objects into renderable ScreenCells
	fn from(input: Tile) -> Self {
		ScreenCell {
			glyph: input.glyph.clone(),
			fg: input.fg,
			bg: input.bg,
			modifier: input.mods,
		}
	}
}
impl From<Vec<String>> for ScreenCell { // Input string should be formatted as "G f b m" where G is the display char and f,b,m are integers
	fn from(input: Vec<String>) -> Self {
		ScreenCell {
			glyph: input[0].clone(),
			fg: input[1].parse::<u8>().unwrap(),
			bg: input[2].parse::<u8>().unwrap(),
			modifier: input[3].parse::<u16>().unwrap()
		}
	}
}
impl From<Vec<&str>> for ScreenCell { // Input string should be formatted as "G f b m" where G is the display char and f,b,m are integers
	fn from(input: Vec<&str>) -> Self {
		ScreenCell {
			glyph: input[0].to_string(),
			fg: input[1].parse::<u8>().unwrap(),
			bg: input[2].parse::<u8>().unwrap(),
			modifier: input[3].parse::<u16>().unwrap()
		}
	}
}
impl ScreenCell {
	/// Creates a ScreenCell from an input string, formatted as "G f b m" where G is the display char,
	/// f and b are the foreground and background colors,
	/// and m is the set of text modifications to apply
	pub fn new_from_str(input: Vec<&str>) -> ScreenCell {
		debug!("* new_from_str input: {:?}", input);
		let mut new_cell = ScreenCell::new();
		new_cell.glyph = input[0].to_string();
		new_cell.fg = input[1].parse::<u8>().unwrap_or_else(|_| COLOR_DICT[input[1]]); // replace 0 with a call to a color name parser
		new_cell.bg = input[2].parse::<u8>().unwrap_or_else(|_| COLOR_DICT[input[2]]);
		new_cell.modifier = input[3].parse::<u16>().unwrap_or(0); // ditto but for modifier names
		new_cell
	}
	pub fn create(new_glyph: &str, new_fg: u8, new_bg: u8, mods: u16) -> ScreenCell {
		ScreenCell {
			glyph: new_glyph.to_string(),
			fg: new_fg,
			bg: new_bg,
			modifier: mods,
		}
	}
	pub fn new() -> ScreenCell {
		ScreenCell::default()
	}
	pub fn glyph(mut self, new_glyph: &str) -> Self {
		self.glyph = new_glyph.to_string();
		self
	}
	pub fn fg(mut self, new_color: u8) -> Self {
		self.fg = new_color;
		self
	}
	pub fn bg(mut self, new_color: u8) -> Self {
		self.bg = new_color;
		self
	}
	pub fn modifier(mut self, new_mod: u16) -> Self {
		self.modifier = new_mod;
		self
	}
	pub fn empty() -> Self {
		ScreenCell {
			glyph: " ".to_string(),
			fg: 8,
			bg: 0,
			modifier: 0,
		}
	}
	pub fn blank() -> Self {
		ScreenCell {
			glyph: "".to_string(),
			fg: 0,
			bg: 0,
			modifier: 0,
		}
	}
	pub fn out_of_bounds() -> Self {
		ScreenCell {
			glyph: "*".to_string(),
			fg: 8,
			bg: 0,
			modifier: 0,
		}
	}
	pub fn fog_of_war() -> Self {
		ScreenCell {
			glyph: " ".to_string(),
			fg: 8,
			bg: 0,
			modifier: 0,
		}
	}
	pub fn placeholder() -> Self {
		ScreenCell {
			glyph: "%".to_string(),
			fg: 5,
			bg: 8,
			modifier: 0,
		}
	}
	pub fn is_blank(&self) -> bool {
		self.glyph == *""
		//&& self.fg == 0
		//&& self.bg == 0
		//&& self.modifier == 0
	}
	pub fn set_glyph(&mut self, new_glyph: &str) {
		self.glyph = new_glyph.to_string();
	}
}

pub fn camera_update_system(mut camera: ResMut<CameraView>,
	                              model: Res<Model>,
	                              p_posn: Res<Position>,
	                              mut p_query:  Query<(Entity, &Body, &Viewshed, &Memory), With<Player>>,
	                              e_query: Query<(Entity, &Body), Without<Player>>,
) {
	// Bail out of the method if we're missing any of the structure we need
	if p_query.get_single_mut().is_err() { return; }
	let (p_enty, p_body, p_viewshed, p_memory) = p_query.get_single_mut().unwrap();
	let world_map = &model.levels[p_posn.z as usize];
	assert!(!camera.output.is_empty(), "camera.output has length 0!");
	assert!(!world_map.tiles.is_empty(), "world_map.tiles has length 0!");
	// Proceed with the update
	let camera_width = camera.width as usize;
	let screen_center = Position::new((camera_width / 2) as i32, camera.height / 2, 0);
	// These map_frame values together define the area of the map that we'll be polling
	let map_frame_ul = Position::new(p_posn.x - screen_center.x, p_posn.y - screen_center.y, 0);
	let map_frame_dr = Position::new(p_posn.x + screen_center.x, p_posn.y + screen_center.y, 0);
	// For every y-position in the map frame and its associated screen position, ...
	for (scr_y, map_y) in (map_frame_ul.y..map_frame_dr.y).enumerate() {
		// For every x-position in the map frame and its associated screen position, ...
		for (scr_x, map_x) in (map_frame_ul.x..map_frame_dr.x).enumerate() {
			//debug!("* scr: {}, {}; map: {}, {}", scr_x, scr_y, map_x, map_y); // DEBUG: print the loop iteration values
			// Get some indices for the various arrays we're going to use
			let scr_index = xy_to_index(scr_x, scr_y, camera_width); // Indexes into the camera's map of the screen
			let map_index = world_map.to_index(map_x, map_y); // Indexes into the worldmap's tilemap
			let map_posn = Position::new(map_x, map_y, p_posn.z); // Shorthand container
			// Check if the map position is currently visible or at least has been seen before
			let is_visible = p_viewshed.visible_points.contains(&Point::new(map_x, map_y));
			let has_seen = if map_index < world_map.revealed_tiles.len() {
				world_map.revealed_tiles[map_index]
			} else {
				false
			};
			// If the map coordinates are valid, and the tile has at least been seen before
			if map_x >= 0 && map_x < world_map.width as i32
			&& map_y >= 0 && map_y < world_map.height as i32
			{
				camera.output[scr_index] = if *p_posn == map_posn { // This is the player's position, always render it
					if let Some(glyph) = p_body.glyph_at(&map_posn) {
						glyph.into()
					} else {
						warn!("Error retrieving player's glyph at the player's position");
						ScreenCell::placeholder()
					}
				}
				else if is_visible { // Not the player, but the player can see it, update accordingly
					// There's no System access over in the WorldMap stuff, so we have to pull the Entity ourselves
					if let Some(enty) = world_map.get_visible_entity_at(map_posn) {
						if enty == p_enty { // If it's the player after all, draw the player
							//debug!("rendering visible entity 'player' at posn {}", map_posn);
							p_body.glyph_at(&map_posn).unwrap().into()
						} else if let Ok((_enty, e_body)) = e_query.get(enty) { // It's a non-player entity
							e_body.glyph_at(&map_posn).unwrap().into()
						} else { // There was somehow a failure to retrieve the visible entity; fallback to the map tile
							//world_map.get_display_tile(map_posn).into() // DEBUG: disabled so i can catch this error case
							warn!("Error retrieving visible entity {:?} from the e_query during camera_update_system at posn {}", enty, map_posn);
							ScreenCell::placeholder()
						}
					} else { // There were no visible entities at the specified position, use a map tile instead
						world_map.get_display_tile(map_posn).into()
					}
				} else if has_seen { // Not the player, not visible, but has been seen by the player in the past
					let mut new_cell: ScreenCell = {
						//debug!("* p_memory.visual dump: {:?}", p_memory.visual); // DEBUG: dump the contents of the Entity's visual memory
						if let Some(enty_list) = p_memory.visual.get(&map_posn) { // Try to get an entity list for that Position
							if !enty_list.is_empty() {
								if let Ok((_, remembered_body)) = e_query.get(enty_list[0]) {
									if let Some(glyph) = remembered_body.glyph_at(&map_posn) {
										glyph.into()
									} else {
										warn!("Error retrieving entity's glyph from e_query during camera_update_system");
										ScreenCell::placeholder()
									}
								} else {
									warn!("Error retrieving remembered entity from the e_query during camera_update_system");
									ScreenCell::placeholder()
								}
							} else { // [1]: There's an entity list but it's empty, so 'fallthru' to the correct case
								// I'm not sure if an actual fallthru is possible, so just make sure this matches the 'else' case below [2]
								//debug!("* tried to get a remembered enty at {:?} but couldn't", map_posn);
								//world_map.get_display_tile(map_posn).into() // DEBUG: disabled so I can see what's being dropped
								ScreenCell::placeholder()
							}
						} else { // [2]: Couldn't get a list -> there's no Entities there -> draw the map Tile instead
							world_map.get_display_tile(map_posn).into()
						}
					};
					new_cell.fg = 8; // Set the foreground to dimmed
					new_cell
				} else { // Player hasn't seen the tile at all, so paint some fog over it
					ScreenCell::fog_of_war()
				}
			} else { // The map coordinates are out of bounds, display a fallback tile
				camera.output[scr_index] = ScreenCell::out_of_bounds(); // Painting this blank tile helps prevent artifacting
			}
			// Paint the targeting reticle onto the map if needed
			if camera.reticle != Position::INVALID {
				// TODO: Add some logic that will detect other entity positions (such as the player!) and choose
				//       a reticle shape that minimizes the number of entities who will be hidden by the points
				// TODO: Add a line-of-sight ruler that can show where the LOS is blocked with line coloration
				let ul_index = xy_to_index(camera.reticle.x as usize - 1, camera.reticle.y as usize - 1, camera_width);
				let ur_index = xy_to_index(camera.reticle.x as usize + 1, camera.reticle.y as usize - 1, camera_width);
				let dl_index = xy_to_index(camera.reticle.x as usize - 1, camera.reticle.y as usize + 1, camera_width);
				let dr_index = xy_to_index(camera.reticle.x as usize + 1, camera.reticle.y as usize + 1, camera_width);
				let ret_chars = camera.reticle_glyphs.clone();
				for (index, corner) in ret_chars.chars().enumerate() {
					match ret_chars.chars().count() {
						3 => { todo!(); /* TODO: impl logic for 3-point reticles */ }
						4 => {
							match index {
								0 => {camera.blinken[ul_index].glyph = corner.to_string(); camera.blinken[ul_index].fg = 11; camera.blinken[ul_index].bg = 8;}
								1 => {camera.blinken[ur_index].glyph = corner.to_string(); camera.blinken[ur_index].fg = 11; camera.blinken[ur_index].bg = 8;}
								2 => {camera.blinken[dl_index].glyph = corner.to_string(); camera.blinken[dl_index].fg = 11; camera.blinken[dl_index].bg = 8;}
								3 => {camera.blinken[dr_index].glyph = corner.to_string(); camera.blinken[dr_index].fg = 11; camera.blinken[dr_index].bg = 8;}
								_ => { }
							}
						}
						_ => { }
					}
				}
			}
		}
	}
}


/* Disabled pending implementation finish
/// Represents a single visual effect to be applied to the CameraView, ie a targeting reticle or explosion effect
#[derive(Component, Resource, Clone, Debug, Default, Reflect)]
pub struct VisualEffect {
	/// How long until the visual effect will be triggered
	pub countdown: i32,
	/// The map position that the effect was triggered at
	pub position: Position,
	/// The list of visual elements that need to be drawn
	/// Each triplet is a single char, plus x and y *offsets*
	pub elements: Vec<(String, i32, i32)>
}
impl VisualEffect { // TODO: add builders to this instead of lumping it into one fxn
	fn new(time: i32, locn: Position, fx: Vec<(String, i32, i32)>) -> Self {
		VisualEffect {
			countdown: time,
			position: locn,
			elements: fx,
		}
	}
}
*/

// Rust all but refuses to allow definition of a static Hashmap, but implementing the color dictionary as a
//   string match is a REALLY fucking stupid idea, I don't know why anyone suggests this shit any more
// Therefore we are forced to rely on an external crate for some fucking idiot reason when this should REALLY
//   be in the stdlib, i don't give two shits about how it breaks the Rust paradigm when *the official docs*
//   recommend installing this fucking crate just to fix an incidental problem
// HEY RUST DEVS, PROTIP: if the official docs are recommending an external solution,
//   then that's a sign that the external solution SHOULD REALLY BE PART OF YOUR STANDARD FUCKING LIBRARY
extern crate lazy_static;
use std::collections::HashMap;
lazy_static::lazy_static! {
	static ref COLOR_DICT: HashMap<&'static str, u8> = {
		let mut map = HashMap::new();
		map.insert("black", 0);
		map.insert("red", 1);
		map.insert("green", 2);
		map.insert("orange", 3);
		map.insert("blue", 4);
		map.insert("purple", 5);
		map.insert("cyan", 6);
		map.insert("white", 7);
		map.insert("grey", 8);
		map.insert("gray", 8);
		map.insert("ltblack", 8);
		map.insert("ltred", 9);
		map.insert("ltgreen", 10);
		map.insert("yellow", 11);
		map.insert("ltblue", 12);
		map.insert("pink", 13);
		map.insert("ltpurple", 13);
		map.insert("ltcyan", 14);
		map.insert("ltwhite", 15);
		map
	};
}
lazy_static::lazy_static! {
	static ref MODS_DICT: HashMap<&'static str, Modifier> = {
		let mut map = HashMap::new();
		map.insert("bright", Modifier::BOLD);
		map.insert("bold", Modifier::BOLD);
		map.insert("dark", Modifier::DIM);
		map.insert("dim", Modifier::DIM);
		map.insert("reverse", Modifier::REVERSED);
		map.insert("underline", Modifier::UNDERLINED);
		map.insert("italic", Modifier::ITALIC);
		map.insert("hidden", Modifier::HIDDEN);
		map.insert("strikeout", Modifier::CROSSED_OUT);
		map.insert("blink", Modifier::SLOW_BLINK);
		map.insert("flash", Modifier::RAPID_BLINK);
		map
	};
}
/// Parses a string of Modifier types into a single value
pub fn parse_mods(input: &str) -> Modifier {
	let tokens: Vec<&str> = input.split(' ').collect();
	let mut modifier = Modifier::empty();
	for string in tokens {
		modifier |= MODS_DICT[string];
	}
	modifier
}

// EOF
