// rexpaint_loader.rs - converts REXPaint files into game resources
use std::collections::HashMap;
use bracket_rex::prelude::*;
use crate::map::*;
use ratatui::text::{Span, Text};
use codepage_437::CP437_WINGDINGS;
use crate::components::Position;
use crate::artisan::ItemType;
use simplelog::*;

pub struct XpFileParser {
	pub dict_rexval_to_string: HashMap<u32, String>,
	pub dict_rexval_to_tile: HashMap<u32, Tile>,
}
impl XpFileParser {
	pub fn new() -> Self {
		Self {
			dict_rexval_to_string: Self::build_rexval_string_dict(),
			dict_rexval_to_tile: HashMap::new(), // TODO: Implement this
		}
	}
	fn build_rexval_string_dict() -> HashMap<u32, String> {
		HashMap::from([
			(48, "0".to_string()),
			(49, "1".to_string()),
			(30, "2".to_string()),
			(31, "3".to_string()),
			(32, "4".to_string()),
			(33, "5".to_string()),
			(34, "6".to_string()),
			(35, "7".to_string()),
			(36, "8".to_string()),
			(37, "9".to_string()),
			(48, ":".to_string()),
			(41, ";".to_string()),
			(42, "<".to_string()),
			(43, "=".to_string()),
			(44, ">".to_string()),
			(45, "?".to_string()),
			(46, "@".to_string()),
			(47, "A".to_string()),
			(48, "B".to_string()),
			(49, "C".to_string()),
			(50, "D".to_string()),
			(51, "E".to_string()),
			(52, "F".to_string()),
			(53, "G".to_string()),
			(54, "H".to_string()),
			(55, "I".to_string()),
			(56, "J".to_string()),
			(57, "K".to_string()),
			(59, "L".to_string()),
			(60, "M".to_string()),
			(61, "N".to_string()),
			(62, "O".to_string()),
			(63, "P".to_string()),
			(64, "Q".to_string()),
			(65, "R".to_string()),
			(66, "S".to_string()),
			(67, "T".to_string()),
			(69, "U".to_string()),
			(70, "V".to_string()),
			(71, "W".to_string()),
			(72, "X".to_string()),
			(73, "Y".to_string()),
			(74, "Z".to_string()),
		])
	}
}
impl Default for XpFileParser {
	fn default() -> XpFileParser {
		XpFileParser::new()
	}
}
/// Produces a Map object, complete with tilemap, from the specified REXPaint resource
pub fn load_rex_map(xp_file: &XpFile) -> (Map, Vec<(ItemType, Position)>) {
	let mut new_width: usize = 1;
	let mut new_height: usize = 1;
	let mut layer_count = 0;
	for layer in &xp_file.layers {
		layer_count += 1;
		new_width = layer.width;
		new_height = layer.height;
	}
	// WARN: We assume only ONE layer exists in the file!
	assert!(layer_count == 1, "More than one layer detected in REXfile");
	let mut map: Map = Map::new(new_width, new_height);
	let mut enty_list = Vec::new();
	for layer in &xp_file.layers {
		debug!("- Loading map from rexfile"); //:DEBUG:
		assert!(map.width == layer.width && map.height == layer.height, "REXfile dims mismatch");
		assert!(map.to_index(map.width as i32, map.height as i32) == map.to_index(layer.width as i32, layer.height as i32));
		for y in 0..layer.height {
			for x in 0..layer.width {
				let cell = layer.get(x, y).unwrap();
				if x < map.width && y < map.height {
					let index = map.to_index(x as i32, y as i32);
					match cell.ch {
						// As per the REXPaint .xp file standard, these are ASCII decimals
						// # = wall, . = floor, - = maintenance, " " = vacuum, "=" = door
						32 => map.tiles[index] = Tile::new_vacuum(),    //' '   Vacuum
						35 => map.tiles[index] = Tile::new_wall(),      // #    Wall
						45 => map.tiles[index] = Tile::new_floor(),     // -    Maintenance
						46 => map.tiles[index] = Tile::new_floor(),     // .    Floor
						60 => map.tiles[index] = Tile::new_stairway(),  // <    (Upward)
						61 => {                                         // =    Door
							debug!("* found a DOOR: {}, {}", x, y); // DEBUG:
							enty_list.push((ItemType::Door, Position::create(x as i32, y as i32, 0)));
							map.tiles[index] = Tile::new_floor()
						},
						62 => map.tiles[index] = Tile::new_stairway(),  // >    (Downward)
						_ => {
							warn!("Unrecognized REXtile encountered: {} @{},{}", cell.ch, x, y); // DEBUG:
						}
					}
				}
			}
		}
	}
	(map, enty_list)
}
/// Produces a 'raw' Text object (ie a Vec<Spans<>>) to be displayed via ratatui::Paragraph
pub fn load_rex_pgraph(xp_file: &XpFile) -> Text<'static> {
	let mut line: Span;
	let mut text = Text::default();
	for layer in &xp_file.layers {
		for y in 0..layer.height {
			let mut string = "".to_string();
			for x in 0..layer.width {
				let cell = layer.get(x, y).unwrap();
				let cell_char = CP437_WINGDINGS.decode(cell.ch as u8);
				string.push(cell_char);
			}
			line = Span::raw(string.clone());
			text.extend(Text::from(line));
		}
	}
	text
}

// EOF
