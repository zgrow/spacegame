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
use ratatui::{
	backend::Backend,
	layout::{
		Constraint,
		Direction,
		Layout,
		Rect
	},
	Terminal,
};

// ###: INTERNAL LIBRARIES
use crate::engine::{AppResult, GameEngine};

//  ###: UIGrid
/// Provides a bunch of named fields (rather than a tuple) of grid components
/// # Fields
/// * `camera_main`     Contains the player's view of the meatspace game world
/// * `msg_world`       Contains the world-level message backlog
/// * `planq_sidebar`   The *entire* PLANQ area, including borders, without subdivisions
/// * `planq_status`    The PLANQ's status bars, at the top
/// * `planq_screen`    The PLANQ's entire terminal view, dynamically sized to leave room for status bars
/// * `planq_stdout`    The part of the _screen that contains the terminal's backscroll
/// * `planq_stdin`     The PLANQ's CLI input box
/// * 'p_status_height' Sets the height of the status bar widget
/// * 'p_stdin_height'  Sets the height of the CLI input widget
pub struct UIGrid {
	/// Provides the main view onto the worldmap
	pub camera_main:      Rect,
	/// Designates the 'default' message log, which always shows msgs from the World channel
	pub msg_world:        Rect,
	/// Designates the area for the whole Planq sidebar, all panels included
	pub planq_sidebar:    Rect,
	/// Designates the space reserved for the Planq's stats: offline status, battery power, &c
	pub planq_status:     Rect,
	/// Designates the space for the Planq's entire terminal
	pub planq_screen:     Rect,
	/// Designates the output screen of the Planq
	pub planq_stdout:     Rect,
	/// Designates the CLI input of the Planq
	pub planq_stdin:      Rect,
	/// Sets the height of the planq_status widget, will be updated during gameplay
	pub p_status_height:  usize,
	/// Sets the height of the planq's CLI widget
	pub p_stdin_height:   usize
}
impl UIGrid {
	pub fn new() -> UIGrid {
		UIGrid {
			camera_main: Rect::default(),
			msg_world: Rect::default(),
			planq_sidebar: Rect::default(),
			planq_status: Rect::default(),
			planq_screen: Rect::default(),
			planq_stdout: Rect::default(),
			planq_stdin: Rect::default(),
			p_status_height: 0,
			p_stdin_height: 1,
		}
	}
	/// Recalculates the PLANQ's layout based on its stored size
	/// Should take into account the dynamic modules, prevent overlap,
	/// and writes its results to the planq_status, planq_screen,
	/// planq_stdout, and planq_stdin fields of the UIGrid object.
	pub fn calc_planq_layout(&mut self, max_area: Rect) {
		// NEW METHOD for PLANQ splits
		// (as a method call somewhere else, so that it can be redone outside of here)
		// given the full width W and height H of the render area,
		// 1- obtain the height of the planq_status module(s), H
		//    (this can be 0 but should be more as the planq_status has some builtins)
		// 2- split H between Max(I) and Min(4) into planq_status and planq_screen,
		//    so that the CLI's stdout will flow to fill the leftover space
		// 3- split planq_screen along the vertical as Min(1), Max(J) where J is the height
		//    of the PLANQ's stdin module, probably = 1 (but not guaranteed!)
		// 4- store these splits on the UI grid:
		//    planq - W, H
		//    \_planq_status - I
		//    \_planq_screen
		//      \_planq_stdout
		//      \_planq_stdin - J
		// ---
		// max_area provides the entire space allowed to this widget
		let first_split = Layout::default()
			.direction(Direction::Vertical)
			.constraints([Constraint::Min(self.p_status_height as u16), Constraint::Min(4)].as_ref())
			.split(max_area).to_vec();
		let second_split = Layout::default()
			.direction(Direction::Vertical)
			.constraints([Constraint::Min(1), Constraint::Max(self.p_stdin_height as u16)].as_ref())
			.split(first_split[1]).to_vec();
		self.planq_status = first_split[0];
		self.planq_screen = first_split[1];
		self.planq_stdout = second_split[0];
		self.planq_stdin = second_split[1];
	}
	/// Recalculates the UI layout based on the given size, to be invoked if the screen is resized
	pub fn calc_layout(&mut self, max_area: Rect) {
		/* Use the layout to build up the UI and its contents
		 * - iterate through the layout stack
		 * - if the object indexed to the layout Rect is active, then draw it
		 * frame.render_widget(self, Widget, area: Rect)
		 * - might consider nesting the calls:
		 *   draw_thing<Backend>(f: &mut Frame<Backend>, app: &mut App, area: Rect)
		 * TODO: one day i'll have the time to make this dynamic/rearrangable...
		 * MAIN LAYOUT
		 * +----+-+
		 * | 1  | |
		 * |    |3|
		 * +----+ |
		 * | 2  | |
		 * +----+-+
		 * block 1 is the overworld camera
		 *  - dims: min: w30, h30, max: fill
		 * block 2 is the PLANQ output and message log
		 *  - dims: min: w(B1), h5+1, max: fill
		 * block 3 is the status output stack
		 *  - layout within block 3 is handled by its internal logic
		 *  - dims: min: w10, h(S), max: w20, h(S)
		 * Cogmind uses a minimum 'grid' size of 80 wide by 60 high, seems legit
		 */
		// Recalculate everything given the new area
		// Split the entire window between [1/2](0) and [3](1) horizontally
		let main_horiz_split = Layout::default()
			.direction(Direction::Horizontal)
			.constraints([Constraint::Min(30), Constraint::Length(32)].as_ref())
			.split(max_area).to_vec();
		// Split [1](0) and [2](1) vertically
		let camera_worldmsg_split = Layout::default()
			.direction(Direction::Vertical)
			.constraints([Constraint::Min(30), Constraint::Length(12)].as_ref())
			.split(main_horiz_split[0]).to_vec();
		// Update the UIGrid itself to hold the new sizes
		self.camera_main = camera_worldmsg_split[0];
		self.msg_world = camera_worldmsg_split[1];
		self.planq_sidebar = main_horiz_split[1];
		self.calc_planq_layout(self.planq_sidebar);
	}
}
impl Default for UIGrid {
	fn default() -> UIGrid {
		UIGrid::new()
	}
}

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
