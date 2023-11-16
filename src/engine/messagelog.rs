// messagelog.rs
// Provides some logical handles to facilitate game logging and display via ratatui

use bevy::prelude::*;
use ratatui::text::{Line, Span};
use ratatui::style::{Style, Color, Modifier};

/// Describes a single entry in the MessageLog; the `text` field supports inline styling, which will be parsed
/// and converted to the appropriate types when ready to be rendered
/// A single Message is roughly equivalent to a ratatui::Line: it can contain multiple spans of styled text,
/// but will not exceed more than one CR/LF
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Resource)]
pub struct Message {
	pub timestamp: i32,
	pub priority: i32,
	pub channel: String,
	pub text: String,
}
impl Message {
	pub fn new(time: i32, level: i32, chan: String, msg: String) -> Message {
		Message {
			timestamp: time,
			priority: level,
			channel: chan,
			text: msg,
		}
	}
}
impl From<Message> for Line<'_> {
	fn from(input: Message) -> Self {
		// SYNTAX
		// enclose the text modifications inside double brackets; fg/bg take color names only
		// "This is some [[fg:red,bg:white,mod:+italic]]red text[[end]]."
		// (end)
		// We can ignore the channel and priority fields because they're for organizational purposes anyway
		// later it might be useful to add some kind of a channel prefix to the message, if so desired
		// -  TODO: Format the timestamp into a suitable prefix
		// -  TODO: Format the priority into a suitable prefix
		// -  TODO: Format the channel into a suitable prefix
		// Parse the text out into raw spans, separated by the inlined control chars
		let mut blocks: Vec<String> = Vec::new(); // The set of substrings that begin with '[['
		let mut line: Vec<Span> = Vec::new();
		// Split the input line into sections that start with control chars
		for chunk in input.text.split("[[") {
			blocks.push(chunk.to_string());
		}
		// For each block of text, ie 'fg:red]]EXIT', 'end]]'
		for block in blocks.iter() {
			let mut style = Style::default();
			if block.is_empty() { continue; } // The leading delimiters cause the split operation to insert empty strings
			let spans = block.split("]]").map(String::from).collect::<Vec<String>>(); // Split each block into two, before/after the control chars
			if spans.len() < 2 { line.push(Span::raw(spans[0].clone())); continue; }
			let trim_chars: &[_] = &['[', ']']; // the split() is supposed to do this, but let's just make sure
			let style_line: Vec<&str> = spans[0].trim_matches(trim_chars).split(',').collect(); // Split the control chars into ind. mods
			// For each individual modification, figure out what type it is and apply it to the Style
			// TODO: make use of the color/modification conversion tools in camera.rs (maybe export them to lib.rs?)
			for token in style_line.iter() {
				let keyval: Vec<&str> = token.split(':').collect();
				match keyval[0] {
					"fg" => {
						match keyval[1] {
							"black"      => { style = style.fg(Color::Black); }
							"red"        => { style = style.fg(Color::Red); }
							"green"      => { style = style.fg(Color::Green); }
							"yellow"     => { style = style.fg(Color::Yellow); }
							"blue"       => { style = style.fg(Color::Blue); }
							"pink"
							| "magenta"
							| "purple"   => { style = style.fg(Color::Magenta); }
							"cyan"       => { style = style.fg(Color::Cyan); }
							"white"      => { style = style.fg(Color::Gray); }
							"ltblack"
							| "grey"
							| "gray"     => { style = style.fg(Color::DarkGray); }
							"ltred"      => { style = style.fg(Color::LightRed); }
							"ltgreen"    => { style = style.fg(Color::LightGreen); }
							"ltyellow"   => { style = style.fg(Color::LightYellow); }
							"ltblue"     => { style = style.fg(Color::LightBlue); }
							"ltpink"
							| "ltmagenta"
							| "ltpurple" => { style = style.fg(Color::LightMagenta); }
							"ltcyan"     => { style = style.fg(Color::LightCyan); }
							"ltwhite"    => { style = style.fg(Color::White); }
							"default"
							| "reset"
							| "end"      => { style = style.fg(Color::Reset); }
							_ => { }
						}
					}
					"bg" => {
						match keyval[1] {
							"black"      => { style = style.bg(Color::Black); }
							"red"        => { style = style.bg(Color::Red); }
							"green"      => { style = style.bg(Color::Green); }
							"yellow"     => { style = style.bg(Color::Yellow); }
							"blue"       => { style = style.bg(Color::Blue); }
							"pink"
							| "magenta"
							| "purple"   => { style = style.bg(Color::Magenta); }
							"cyan"       => { style = style.bg(Color::Cyan); }
							"white"      => { style = style.bg(Color::Gray); }
							"ltblack"
							| "grey"
							| "gray"     => { style = style.bg(Color::DarkGray); }
							"ltred"      => { style = style.bg(Color::LightRed); }
							"ltgreen"    => { style = style.bg(Color::LightGreen); }
							"ltyellow"   => { style = style.bg(Color::LightYellow); }
							"ltblue"     => { style = style.bg(Color::LightBlue); }
							"ltpink"
							| "ltmagenta"
							| "ltpurple" => { style = style.bg(Color::LightMagenta); }
							"ltcyan"     => { style = style.bg(Color::LightCyan); }
							"ltwhite"    => { style = style.bg(Color::White); }
							"default"
							| "reset"
							| "end"      => { style = style.bg(Color::Reset); }
							_ => { }
						}
					}
					"mod" => {
						// need to do some special splitting and parsing here
						let mut pos_mods = Modifier::empty();
						let mut neg_mods = Modifier::empty();
						let mods: Vec<&str> = keyval[1].split('/').collect();
						for element in mods.iter() {
							let mut token = element.to_string();
							let polarity = token.remove(0); // get the first char off the element
							let bit_mod = match &*token { // Arranged in order of descending support; blink/flash and strikeout esp. are rare
								"bright"
								| "bold"    => { Modifier::BOLD }
								"dark"
								| "dim"     => { Modifier::DIM }
								"reverse"   => { Modifier::REVERSED }
								"underline" => { Modifier::UNDERLINED }
								"italic"    => { Modifier::ITALIC }
								"hidden"    => { Modifier::HIDDEN }
								"strikeout" => { Modifier::CROSSED_OUT }
								"blink"     => { Modifier::SLOW_BLINK }
								"flash"     => { Modifier::RAPID_BLINK }
								_ => { Modifier::empty() }
							};
							if polarity == '+' {
								pos_mods |= bit_mod;
							} else if polarity == '-' {
								neg_mods |= bit_mod;
							} else {
								error!("* ERR: color parse failure, unsupported mod: {}{}", polarity, element);
							}
							// Apply the bitfield modifiers, if any
						}
						if pos_mods != Modifier::empty() {
							style = style.add_modifier(pos_mods);
						}
						if neg_mods != Modifier::empty() { style = style.remove_modifier(neg_mods); }
					}
					"default" | "reset" | "end" => {
						style = Style::reset();
					}
					_ => { }
				}
			}
			let new_span = Span::styled(spans[1].clone(), style);
			line.push(new_span);
		}
		Line::from(line)
	}
}
#[derive(Resource, Clone, Debug, Default, PartialEq, Reflect)]
//#[reflect(Resource)]
pub struct MessageChannel {
	pub name: String,
	pub contents: Vec<Message>,
}
impl MessageChannel {
	pub fn new(new_name: &String) -> MessageChannel {
		MessageChannel {
			name: new_name.to_string(),
			contents: Vec::new(),
		}
	}
	pub fn add(&mut self, new_msg: Message) {
		self.contents.push(new_msg);
	}
	pub fn pop(&mut self) -> Option<Message> {
		self.contents.pop()
	}
}
#[derive(Resource, Clone, Debug, Default, PartialEq, Reflect)]
#[reflect(Resource)]
pub struct MessageLog {
	pub logs: Vec<MessageChannel>
}
impl MessageLog {
	/// Creates a new MessageLog with the preset channels
	pub fn new(channels: Vec<String>) -> MessageLog {
		let mut new_logs = Vec::new();
		for name in channels {
			new_logs.push(MessageChannel::new(&name));
		}
		MessageLog{ logs: new_logs }
	}
	//  * TOOLS
	/// Adds a new message to the given channel; if the channel does not exist it will be made
	/// # Arguments
	/// * `msg_text` - The text of the message
	/// * `msg_chan` - The msg channel's name, ie "world"
	/// * `msg_prio` - Higher -> more important
	/// * `msg_time` - As number of seconds since game epoch
	pub fn add(&mut self, msg_text: String, msg_chan: String, msg_prio: i32, msg_time: i32) {
		// Check for an existing channel to add the new message to
		for channel in &mut self.logs {
			if channel.name == msg_chan {
				// add the message to this channel
				channel.add(Message::new(msg_time, msg_prio, msg_chan, msg_text));
				return;
			}
		}
		// if we arrived here, we didn't find a matching channel
		// make a new channel and add the message to it
		let mut new_channel = MessageChannel::new(&msg_chan);
		new_channel.add(Message::new(msg_time, msg_prio, msg_chan, msg_text));
		self.logs.push(new_channel);
	}
	/// Replaces the last message in the given channel with the new message; does nothing if channel does not exist
	pub fn replace(&mut self, msg_text: String, msg_chan: String, msg_prio: i32, msg_time: i32) {
		// Check for an existing channel to add the new message to
		for channel in &mut self.logs {
			if channel.name == msg_chan {
				// add the message to this channel
				channel.pop();
				channel.add(Message::new(msg_time, msg_prio, msg_chan, msg_text));
				return;
			}
		}
		// if we arrived here, we didn't find a matching channel, don't do anything
	}
	/// Counts the number of messages in the specified channel; RETURNS 0 if channel not found!
	pub fn channel_len(&self, req_channel: String) -> usize {
		for channel in &self.logs {
			if channel.name == req_channel { return channel.contents.len(); }
		}
		0
	}
	/// Sends a boot message associated with the given boot_stage to the PLANQ's channel
	pub fn boot_message(&mut self, boot_stage: u32) {
		if boot_stage > 4 {
			return;
		}
		match boot_stage {
			// This version of the OS logo doesn't have the extra \s, which are required as escapes by Rust
			//                     ▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄
			//                     ▌ __         __  __     __   ▐
			//                     ▌/   _||   |/  \(_     /_    ▐
			//                     ▌\__(-|||_||\__/__)  \/__)/) ▐
			//                     ▌────────<-──────────<-─<{ (<▐
			//                     ▌         \           \   \) ▐
			//                     ▙▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▟
			//                     _123456789_12356789_123456789_
			0 => {
				//│─
				self.tell_planq("[[fg:gray]]╃────────────────────────────╄".to_string());
				self.tell_planq("[[fg:gray]]│[[fg:ltcyan]] __         __  __     __   [[fg:gray]]│".to_string());
				self.tell_planq("[[fg:gray]]│[[fg:ltcyan]]/   _||   |/  \\(_     /_    [[fg:gray]]│".to_string());
				self.tell_planq("[[fg:gray]]│[[fg:ltcyan]]\\__(-|||_||\\__/__)  [[fg:green]]\\/[[fg:ltcyan]]__)[[fg:red]]/) [[fg:gray]]│".to_string());
				self.tell_planq("[[fg:gray]]│[[fg:green]]────────<-──────────<-─<[[fg:red]]{ (<[[fg:gray]]│".to_string());
				self.tell_planq("[[fg:gray]]│[[fg:green]]         \\           \\   [[fg:red]]\\) [[fg:gray]]│".to_string());
				self.tell_planq("[[fg:gray]]┽────────────────────────────╆".to_string());
				self.tell_planq(" ".to_string());
				self.tell_planq("[[fg:yellow]]¶[[fg:gray]]│[[end]]BIOS:  GRAIN v17.6.8, [[mod:+italic]]Cedar[[end]]".to_string());
			}
			1 => {
				self.tell_planq("[[fg:yellow]]¶[[fg:gray]]│[[end]]Hardware Status ..... [ [[fg:green]]OK[[end]] ]".to_string());
			}
			2 => {
				self.tell_planq("[[fg:yellow]]¶[[fg:gray]]│[[end]]Firmware Status ..... [ [[fg:green]]OK[[end]] ]".to_string());
			}
			3 => {
				self.tell_planq("[[fg:yellow]]¶[[fg:gray]]│[[end]]Bootloader Status ... [ [[fg:green]]OK[[end]] ]".to_string());
			}
			4 => {
				self.tell_planq("[[fg:yellow]]¶[[fg:gray]]│[[end]]Ready for input!".to_string());
			}
			_ => { }
		};
	}
	/// Clears a message channel's backscroll: WARN: irreversible!
	/// Returns false if the specified channel was not found
	pub fn clear(&mut self, target: String) -> bool {
		if let Some(chan_index) = self.logs.iter().position(|x| x.name == target) {
			self.logs[chan_index].contents.clear();
			return true;
		}
		false
	}
	/// Retrieves a set of log messages from a specified channel as ratatui::Line
	/// This means the text will be formatted for display in a ratatui::Paragraph!
	/// If the given channel does not exist, an empty vector will be returned
	/// Specify a count of 0 to obtain the full log for that channel
	pub fn get_log_as_lines(&self, req_channel: String, count: usize) -> Vec<Line> {
		// TODO: See if possible to optimize this by not building the whole list each time
		let mut backlog: Vec<Line> = Vec::new();
		if self.logs.is_empty() { return backlog; }
		for channel in &self.logs {
			if channel.name == req_channel {
				for msg in &channel.contents {
					backlog.push(msg.clone().into());
				}
			}
		}
		if count != 0 {
			let offset = backlog.len() - count;
			backlog = backlog[offset..].to_vec();
		}
		backlog
	}
	/// Retrieves a set of log messages from a specified channel as my Message object
	/// This preserves the log message metadata
	/// If the given channel does not exist, an empty vector will be returned
	/// Specify a count of 0 to obtain the full log for that channel
	pub fn get_log_as_messages(&self, req_channel: String, count: usize) -> Vec<Message> {
		if self.logs.is_empty() { return Vec::new(); }
		for channel in &self.logs {
			if channel.name == req_channel {
				if count == 0 { return channel.contents.clone(); }
				let offset = channel.contents.len() - count;
				return channel.contents[offset..].to_vec();
			}
		}
		Vec::new()
	}
	/// Helper method for writing a message directly to the "world" channel, ie the main feedback message channel
	pub fn tell_player(&mut self, msg_text: String) {
		self.add(msg_text, "world".to_string(), 0, 0);
	}
	/// Helper method: adds a new message directly to the "planq" channel (aka 'stdout')
	pub fn tell_planq(&mut self, msg_text: String) {
		self.add(msg_text, "planq".to_string(), 0, 0);
	}

}
/// Implements the Default trait for the reference type
impl<'a> Default for &'a MessageLog {
	fn default() -> &'a MessageLog {
		static VALUE: MessageLog = MessageLog {
			logs: Vec::new(),
		};
		&VALUE
	}
}

// EOF
