// engine/tui.rs
// July 12 2023
// File was cribbed/copied from orhun/rust-tui-template output

// ###: EXTERNAL LIBRARIES
use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use crossterm::event::{
	self,
	Event as CrosstermEvent,
	KeyEvent,
	MouseEvent,
	DisableMouseCapture,
	EnableMouseCapture,
};
use crossterm::terminal::{
	self,
	EnterAlternateScreen,
	LeaveAlternateScreen,
};
use ratatui::backend::Backend;
use ratatui::Terminal;

// ###: INTERNAL LIBRARIES
use crate::engine::{AppResult, GameEngine};

//  ###: Tui
/// Provides the representation of the TUI, sets up the terminal and interface, handles drawing events
#[derive(Debug)]
pub struct Tui<B: Backend> {
	/// Interface to the Terminal.
	terminal: Terminal<B>,
	/// Terminal event handler.
	pub events: TuiEventHandler,
}
impl<B: Backend> Tui<B> {
	/// Constructs a new instance of [`Tui`].
	pub fn new(terminal: Terminal<B>, events: TuiEventHandler) -> Self {
		Self { terminal, events }
	}
	/// Initializes the terminal interface.
	///
	/// It enables the raw mode and sets terminal properties.
	pub fn init(&mut self) -> AppResult<()> {
		terminal::enable_raw_mode()?;
		crossterm::execute!(io::stderr(), EnterAlternateScreen, EnableMouseCapture)?;
		self.terminal.hide_cursor()?;
		self.terminal.clear()?;
		Ok(())
	}
	/// [`Draw`] the terminal interface by [`rendering`] the widgets.
	///
	/// [`Draw`]: tui::Terminal::draw
	/// [`rendering`]: crate::app::GameEngine::render
	pub fn draw(&mut self, app: &mut GameEngine) -> AppResult<()> {
		self.terminal.draw(|frame| app.render(frame))?;
		Ok(())
	}
	/// Exits the terminal interface.
	///
	/// It disables the raw mode and reverts back the terminal properties.
	pub fn exit(&mut self) -> AppResult<()> {
		terminal::disable_raw_mode()?;
		crossterm::execute!(io::stderr(), LeaveAlternateScreen, DisableMouseCapture)?;
		self.terminal.show_cursor()?;
		Ok(())
	}
}
//  ###: TuiEventHandler
/// Handles the TUI events
#[allow(dead_code)]
#[derive(Debug)]
pub struct TuiEventHandler {
	/// Event sender channel.
	sender: mpsc::Sender<TuiEvent>,
	/// Event receiver channel.
	receiver: mpsc::Receiver<TuiEvent>,
	/// Event handler thread.
	handler: thread::JoinHandle<()>,
}
impl TuiEventHandler {
	/// Constructs a new instance of [`EventHandler`].
	pub fn new(tick_rate: u64) -> Self {
		let tick_rate = Duration::from_millis(tick_rate);
		let (sender, receiver) = mpsc::channel();
		let handler = {
			let sender = sender.clone();
			thread::spawn(move || {
				let mut last_tick = Instant::now();
				loop {
					let timeout = tick_rate
						.checked_sub(last_tick.elapsed())
						.unwrap_or(tick_rate);
					if event::poll(timeout).expect("no events available") {
						match event::read().expect("unable to read event") {
							CrosstermEvent::Key(e) => sender.send(TuiEvent::Key(e)),
							CrosstermEvent::Mouse(e) => sender.send(TuiEvent::Mouse(e)),
							CrosstermEvent::Resize(w, h) => sender.send(TuiEvent::Resize(w, h)),
							_ => unimplemented!(),
						}
						.expect("failed to send terminal event")
					}
					if last_tick.elapsed() >= tick_rate {
						sender.send(TuiEvent::Tick).expect("failed to send tick event");
						last_tick = Instant::now();
					}
				}
			})
		};
		Self {
			sender,
			receiver,
			handler,
		}
	}
	/// Receive the next event from the handler thread.
	///
	/// This function will always block the current thread if
	/// there is no data available and it's possible for more data to be sent.
	pub fn next(&self) -> AppResult<TuiEvent> {
		Ok(self.receiver.recv()?)
	}
}
//  ###: TuiEvent
/// Defines the set of interface events in the TUI
#[derive(Clone, Copy, Debug)]
pub enum TuiEvent {
	/// One tick of the game engine
	Tick,
	/// A key press
	Key(KeyEvent),
	/// A mouse click or scroll
	Mouse(MouseEvent),
	/// Terminal has been resized
	Resize(u16, u16)
}

// EOF
