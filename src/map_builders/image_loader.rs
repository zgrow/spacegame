// image_loader.rs - convers REXPaint files into game resources
use bracket_rex::prelude::*;
use crate::map::*;

pub fn load_rex_map(new_depth: i32, xp_file: &XpFile) -> Map {
	let mut new_width: i32 = 1;
	let mut new_height: i32 = 1;
	let mut layer_count = 0;
	for layer in &xp_file.layers {
		layer_count += 1;
		new_width = layer.width as i32;
		new_height = layer.height as i32;
	}
	assert!(layer_count == 1, "More than one layer detected in REXfile");
	let mut map: Map = Map::new(new_depth, new_width, new_height);
	// WARN: This assumes only ONE layer exists in the file!
	for layer in &xp_file.layers {
		//eprintln!("- Loading map from rexfile"); //:DEBUG:
		assert!(map.width == layer.width as i32 && map.height == layer.height as i32, "REXfile dims mismatch");
		assert!(map.to_index(map.width, map.height) == map.to_index(layer.width as i32, layer.height as i32));
		for y in 0..layer.height {
			for x in 0..layer.width {
				let cell = layer.get(x, y).unwrap();
				if x < map.width as usize && y < map.height as usize {
					let index = map.to_index(x as i32, y as i32);
					match cell.ch {
						// As per the REXPaint .xp file standard, these are ASCII decimals
						// # = wall, . = floor, - = maintenance, " " = vacuum, "=" = door
						32 => map.tiles[index] = Tile::new_vacuum(),//' '   Vacuum
						35 => map.tiles[index] = Tile::new_wall(),  // #    Wall
						45 => map.tiles[index] = Tile::new_floor(), // -    Maintenance
						46 => map.tiles[index] = Tile::new_floor(), // .    Floor
						60 => map.tiles[index] = Tile::new_floor(), // <    (Upward)
						61 => map.tiles[index] = Tile::new_floor(), // =    Door
						62 => map.tiles[index] = Tile::new_floor(), // >    (Downward)
						_ => {
							//eprintln!("Unrecognized REXtile encountered: {} @{},{}", cell.ch, x, y);
						}
					}
				}
			}
		}
	}
	map
}

// EOF
