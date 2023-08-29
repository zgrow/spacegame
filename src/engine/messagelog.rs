// messagelog.rs
// Provides some logical handles to facilitate game logging and display via ratatui

use bevy::prelude::*;
use ratatui::text::Line;

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
			0 => {self.tell_planq("¶│BIOS:  GRAIN v17.6.8 'Cedar'".to_string());}
			1 => {self.tell_planq("¶│Hardware Status ....... [OK]".to_string());}
			2 => {self.tell_planq("¶│Firmware Status ....... [OK]".to_string());}
			3 => {self.tell_planq("¶│Bootloader Status ..... [OK]".to_string());}
			4 => {self.tell_planq("▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄".to_string());
						self.tell_planq("▌ __         __  __     __   ▐".to_string());
						self.tell_planq("▌/   _||   |/  \\(_     /_    ▐".to_string());
						self.tell_planq("▌\\__(-|||_||\\__/__)  \\/__)/) ▐".to_string());
						self.tell_planq("▌────────<-──────────<-─<{ (<▐".to_string());
						self.tell_planq("▌         \\           \\   \\) ▐".to_string());
						self.tell_planq("▙▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▟".to_string());
						self.tell_planq(" ".to_string());
						self.tell_planq("¶│Ready for input!".to_string());
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
					backlog.push(msg.text.clone().into());
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
