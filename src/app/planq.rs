// planq.rs
// Provides the handling and abstractions for the player's PLANQ

use ratatui::buffer::*;
use ratatui::layout::*;
use ratatui::widgets::*;
use ratatui::style::*;

pub struct Planq<'a> {
	data: Vec<String>,
	block: Option<Block<'a>>,
	style: Style,
	align: Alignment,
}
impl<'a> Widget for Planq<'a> {
	fn render(mut self, area: Rect, buf: &mut Buffer) {
		// Draw the border, if it exists
		let area = match self.block.take() {
			Some(b) => {
				let inner_area = b.inner(area);
				b.render(area, buf);
				inner_area
			}
			None => area,
		};
		// area now contains the remaining space to draw the PLANQ
		// anything wider than this is going to get truncated!
		let max_width = area.right() - area.left();
		// The top and bottom panes are 'fixed' size, while the middle pane is expandable
		// TODO: The middle pane should be 'smart', and can count how many slots it has available
		//       for the player to load things into
		let textstyle = Style::default().fg(Color::White);
		let layout = Layout::default()
			.direction(Direction::Vertical)
			.constraints([Constraint::Length(20), Constraint::Min(3), Constraint::Length(20)].as_ref())
			.split(area).to_vec();
		eprintln!("{}, {}", max_width, layout.len());
		// (test) draw a couple of internal borders
		for x_index in layout[0].left()..layout[0].right() {
			buf.set_string(x_index, layout[0].bottom(), "-".to_string(), textstyle);
			buf.set_string(x_index, layout[1].bottom(), "-".to_string(), textstyle);
		}
		// put the contents of self.data on the screen
		let mut y_index = layout[0].top();
		for line in self.data {
			buf.set_string(layout[0].left(), y_index, line, textstyle);
			y_index += 1;
		}
	}
}
impl<'a> Planq<'a> {
	pub fn new(new_data: &'a Vec<String>) -> Planq<'a> {
		Planq {
			data: new_data.to_vec(),
			block: None,
			style: Style::default(),
			align: Alignment::Left,
		}
	}
	pub fn block(mut self, block: Block<'a>) -> Planq<'a> {
		self.block = Some(block);
		self
	}
	pub fn style(mut self, style: Style) -> Planq<'a> {
		self.style = style;
		self
	}
	pub fn alignment(mut self, align: Alignment) -> Planq<'a> {
		self.align = align;
		self
	}

}
