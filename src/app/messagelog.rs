// messagelog.rs
// Provides some logical handles to facilitate game logging and display via ratatui

use bevy::ecs::system::Resource;

#[derive(PartialEq, Clone)]
pub struct Message {
	timestamp: i32,
	priority: i32,
	channel: String,
	text: String,
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
#[derive(PartialEq, Clone)]
pub struct MessageChannel {
	name: String,
	contents: Vec<Message>,
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
#[derive(PartialEq, Clone, Resource)]
pub struct MessageLog {
	logs: Vec<MessageChannel>
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
	/// Adds a new message to the given channel; if the channel does not exist it will be made
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
	/// Retrieves the set of log messages from a specified channel
	/// FIXME: this crashes if the specified channel doesn't exist!
	pub fn get_log(&self, req_channel: String) -> Vec<String> {
		let mut backlog = Vec::new();
		for channel in &self.logs {
			if channel.name == req_channel {
				for msg in &channel.contents {
					backlog.push(msg.text.clone());
				}
			}
		}
		return backlog;
	}
}

// EOF
