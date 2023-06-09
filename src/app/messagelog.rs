// messagelog.rs
// Provides some logical handles to facilitate game logging and display via ratatui

use ratatui::text::Spans;

use bevy::prelude::*;
#[derive(PartialEq, Eq, Clone, Reflect, FromReflect, Debug)]
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
#[derive(PartialEq, Clone, Reflect, FromReflect)]
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
}
#[derive(PartialEq, Clone, Resource, Reflect, Default)]
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
	/// Counts the number of messages in the specified channel; RETURNS 0 if channel not found!
	pub fn channel_len(&self, req_channel: String) -> usize {
		for channel in &self.logs {
			if channel.name == req_channel { return channel.contents.len(); }
		}
		0
	}
	/// Helper method: adds a new message directly to the "world" channel [TODO: with an immediate timestamp]
	pub fn tell_player(&mut self, msg_text: String) {
		self.add(msg_text, "world".to_string(), 0, 0);
	}
	/// Helper method: adds a new message directly to the "planq" channel (aka 'stdout')
	pub fn tell_planq(&mut self, msg_text: String) {
		self.add(msg_text, "planq".to_string(), 0, 0);
	}
	/// Retrieves a set of log messages from a specified channel as ratatui::Spans
	/// This means the text will be formatted for display in a ratatui::Paragraph!
	/// If the given channel does not exist, an empty vector will be returned
	/// Specify a count of 0 to obtain the full log for that channel
	pub fn get_log_as_spans(&self, req_channel: String, count: usize) -> Vec<Spans> {
		// TODO: See if possible to optimize this by not building the whole list each time
		let mut backlog: Vec<Spans> = Vec::new();
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
