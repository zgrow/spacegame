// rex_assets.rs - provides methods for handling REXPaint files
use bracket_rex::prelude::*;
use bracket_embedding::prelude::*;
use bevy::ecs::prelude::*;

// TO ADD A NEW RESOURCE, follow the numbers:
// 1: Add embedded_resource!s here
//embedded_resource!(TEST_SHIP, "../resources/test_ship_v2.xp");
embedded_resource!(PAUSE_GRAPHIC, "../resources/big_pause.xp");
#[derive(Resource)]
pub struct RexAssets {
	// 2: Each new resource object gets a named entry in this list...
	//pub test_map: XpFile,
	pub pause_banner: XpFile
}
impl RexAssets {
	#[allow(clippy::new_without_default)]
	pub fn new() -> RexAssets {
		// 3: And a link_resource to the above embed...
		//link_resource!(TEST_SHIP, "../resources/test_ship.xp");
		link_resource!(PAUSE_GRAPHIC, "../resources/big_pause.xp");
		RexAssets {
			// 4: And finally, populate the resource with the file's data
			//test_map: XpFile::from_resource("../resources/test_ship.xp").unwrap(),
			pause_banner: XpFile::from_resource("../resources/big_pause.xp").unwrap()
		}
	}
}

// EOF
