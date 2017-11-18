use std;
use conrod;
use find_folder;
use conrod::backend::glium::glium::{self, Surface};

const WIN_W: u32 = 600;
const WIN_H: u32 = 400;

pub struct AppState {
	command_input_text: String,
}

impl AppState {
	pub fn new() -> Self {
		AppState {
			command_input_text: String::from("hhhhhhhhhhhh"),
		}
	}
}

pub fn show() {
	let mut events_loop = glium::glutin::EventsLoop::new();
	let window = glium::glutin::WindowBuilder::new()
		.with_title("Verco")
		.with_dimensions(WIN_W, WIN_H);
	let context = glium::glutin::ContextBuilder::new()
		.with_vsync(true)
		.with_multisampling(4);
	let display = glium::Display::new(window, context, &events_loop).unwrap();

	let mut ui = conrod::UiBuilder::new([WIN_W as f64, WIN_H as f64])
		.theme(theme())
		.build();

	let ids = Ids::new(ui.widget_id_generator());

	let assets = find_folder::Search::KidsThenParents(3, 5)
		.for_folder("assets")
		.unwrap();
	let font_path = assets.join("fonts/NotoSans/NotoSans-Regular.ttf");
	ui.fonts.insert_from_file(font_path).unwrap();

	let image_map = conrod::image::Map::<conrod::glium::Texture2d>::new();

	let mut app = AppState::new();
	let mut renderer = conrod::backend::glium::Renderer::new(&display).unwrap();

	// Start the loop:
 //
 // - Poll the window for available events.
 // - Update the widgets via the `gui` fn.
 // - Render the current state of the `Ui`.
 // - Repeat.
	let mut event_loop = EventLoop::new();
	'main: loop {
		// Handle all events.
		for event in event_loop.next(&mut events_loop) {
			// Use the `winit` backend feature to convert the winit event to a conrod one.
			if let Some(event) = conrod::backend::winit::convert_event(event.clone(), &display) {
				ui.handle_event(event);
				event_loop.needs_update();
			}

			match event {
				glium::glutin::Event::WindowEvent { event, .. } => match event {
					glium::glutin::WindowEvent::Closed |
					glium::glutin::WindowEvent::KeyboardInput {
						input:
							glium::glutin::KeyboardInput {
								virtual_keycode: Some(glium::glutin::VirtualKeyCode::Escape),
								..
							},
						..
					} => break 'main,
					_ => (),
				},
				_ => (),
			}
		}

		// Instantiate a GUI demonstrating every widget type provided by conrod.
		gui(&mut ui.set_widgets(), &ids, &mut app);

		display
			.gl_window()
			.window()
			.set_cursor(conrod::backend::winit::convert_mouse_cursor(
				ui.mouse_cursor(),
			));

		if let Some(primitives) = ui.draw_if_changed() {
			renderer.fill(&display, primitives, &image_map);
			let mut target = display.draw();
			target.clear_color(0.0, 0.0, 0.0, 1.0);
			renderer.draw(&display, &mut target, &image_map).unwrap();
			target.finish().unwrap();
		}
	}
}

widget_ids! {
	pub struct Ids {
		canvas,

		command_name,
		command_input,
		command_output,

		canvas_scrollbar,
	}
}

struct EventLoop {
	ui_needs_update: bool,
	last_update: std::time::Instant,
}

impl EventLoop {
	pub fn new() -> Self {
		EventLoop {
			last_update: std::time::Instant::now(),
			ui_needs_update: true,
		}
	}

	/// Produce an iterator yielding all available events.
	pub fn next(
		&mut self,
		events_loop: &mut glium::glutin::EventsLoop,
	) -> Vec<glium::glutin::Event> {
		// We don't want to loop any faster than 60 FPS, so wait until it has been at least 16ms
  // since the last yield.
		let last_update = self.last_update;
		let sixteen_ms = std::time::Duration::from_millis(16);
		let duration_since_last_update = std::time::Instant::now().duration_since(last_update);
		if duration_since_last_update < sixteen_ms {
			std::thread::sleep(sixteen_ms - duration_since_last_update);
		}

		// Collect all pending events.
		let mut events = Vec::new();
		events_loop.poll_events(|event| events.push(event));

		// If there are no events and the `Ui` does not need updating, wait for the next event.
		if events.is_empty() && !self.ui_needs_update {
			events_loop.run_forever(|event| {
				events.push(event);
				glium::glutin::ControlFlow::Break
			});
		}

		self.ui_needs_update = false;
		self.last_update = std::time::Instant::now();

		events
	}

	/// Notifies the event loop that the `Ui` requires another update whether or not there are any
	/// pending events.
	///
	/// This is primarily used on the occasion that some part of the `Ui` is still animating and
	/// requires further updates to do so.
	pub fn needs_update(&mut self) {
		self.ui_needs_update = true;
	}
}

fn theme() -> conrod::Theme {
	use conrod::position::{Align, Direction, Padding, Position, Relative};
	conrod::Theme {
		name: "Demo Theme".to_string(),
		padding: Padding::none(),
		x_position: Position::Relative(Relative::Align(Align::Start), None),
		y_position: Position::Relative(Relative::Direction(Direction::Backwards, 20.0), None),
		background_color: conrod::color::DARK_CHARCOAL,
		shape_color: conrod::color::LIGHT_CHARCOAL,
		border_color: conrod::color::BLACK,
		border_width: 0.0,
		label_color: conrod::color::WHITE,
		font_id: None,
		font_size_large: 26,
		font_size_medium: 18,
		font_size_small: 12,
		widget_styling: conrod::theme::StyleMap::default(),
		mouse_drag_threshold: 0.0,
		double_click_threshold: std::time::Duration::from_millis(500),
	}
}

pub fn gui(ui: &mut conrod::UiCell, ids: &Ids, app: &mut AppState) {
	use conrod::{color, widget, Colorable, Positionable, Sizeable, Widget};

	widget::Canvas::new()
		.pad(30.0)
		//.scroll_kids_vertically()
		.set(ids.canvas, ui);

	widget::Text::new("status")
		.font_size(42)
		.top_left_of(ids.canvas)
		.left_justify()
		.set(ids.command_name, ui);

	for edit in widget::TextEdit::new(&mut app.command_input_text)
		.color(color::WHITE)
		//.padded_w_of(ids.canvas, 20.0)
		//.mid_top_of(ids.canvas)
		.left_justify()
		.restrict_to_height(false)
		.set(ids.command_input, ui)
	{
		app.command_input_text = edit;
	}

	widget::Text::new(&String::from("nothing to commit\nor has it?\n\ndone!")[..])
		.font_size(12)
		//.align_middle_x_of(ids.canvas)
		.left_justify()
		.line_spacing(5.0)
		.set(ids.command_output, ui);
}
