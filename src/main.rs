use glyphon::{
	Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, Resolution,
	Shaping, SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer,
	Viewport,
};
use std::sync::Arc;
use wgpu::{
	CommandEncoderDescriptor, CompositeAlphaMode, DeviceDescriptor, Instance,
	InstanceDescriptor, LoadOp, MultisampleState, Operations, PresentMode,
	RenderPassColorAttachment, RenderPassDescriptor, RequestAdapterOptions,
	SurfaceConfiguration, TextureFormat, TextureUsages, TextureViewDescriptor,
};
use winit::{
	event::{ElementState, WindowEvent},
	event_loop::EventLoop,
	keyboard::{Key, ModifiersState, NamedKey},
	platform::modifier_supplement::KeyEventExtModifierSupplement as _,
	window::Window,
};

fn main() {
	let event_loop = EventLoop::new().unwrap();
	event_loop
		.run_app(&mut Application { window_state: None })
		.unwrap();
}

struct WindowState {
	device: wgpu::Device,
	queue: wgpu::Queue,
	surface: wgpu::Surface<'static>,
	surface_config: SurfaceConfiguration,

	font_system: FontSystem,
	swash_cache: SwashCache,
	viewport: glyphon::Viewport,
	atlas: glyphon::TextAtlas,
	text_renderer: glyphon::TextRenderer,
	text_buffer: glyphon::Buffer,
	text: String,

	// Make sure that the winit window is last in the struct so that
	// it is dropped after the wgpu surface is dropped, otherwise the
	// program may crash when closed. This is probably a bug in wgpu.
	window: Arc<Window>,
	modifiers: ModifiersState,
}

impl WindowState {
	async fn new(window: Arc<Window>) -> Self {
		let physical_size = window.inner_size();
		let scale_factor = window.scale_factor();

		// Set up surface
		let instance = Instance::new(&InstanceDescriptor::default());
		let adapter = instance
			.request_adapter(&RequestAdapterOptions::default())
			.await
			.unwrap();
		let (device, queue) = adapter
			.request_device(&DeviceDescriptor::default())
			.await
			.unwrap();

		let surface = instance
			.create_surface(window.clone())
			.expect("Create surface");
		let swapchain_format = TextureFormat::Bgra8UnormSrgb;
		let surface_config = SurfaceConfiguration {
			usage: TextureUsages::RENDER_ATTACHMENT,
			format: swapchain_format,
			width: physical_size.width,
			height: physical_size.height,
			present_mode: PresentMode::Fifo,
			alpha_mode: CompositeAlphaMode::Opaque,
			view_formats: vec![],
			desired_maximum_frame_latency: 2,
		};
		surface.configure(&device, &surface_config);

		// Set up text renderer
		let mut font_system = FontSystem::new();
		let swash_cache = SwashCache::new();
		let cache = Cache::new(&device);
		let viewport = Viewport::new(&device, &cache);
		let mut atlas =
			TextAtlas::new(&device, &queue, &cache, swapchain_format);
		let text_renderer = TextRenderer::new(
			&mut atlas,
			&device,
			MultisampleState::default(),
			None,
		);
		let mut text_buffer =
			Buffer::new(&mut font_system, Metrics::new(30.0, 42.0));

		let physical_width = (physical_size.width as f64 * scale_factor) as f32;
		let physical_height =
			(physical_size.height as f64 * scale_factor) as f32;

		text_buffer.set_size(
			&mut font_system,
			Some(physical_width),
			Some(physical_height),
		);
		text_buffer.set_text(
			&mut font_system,
			"",
			&Attrs::new().family(Family::SansSerif),
			Shaping::Advanced,
		);
		text_buffer.shape_until_scroll(&mut font_system, false);

		Self {
			device,
			queue,
			surface,
			surface_config,
			font_system,
			swash_cache,
			viewport,
			atlas,
			text_renderer,
			text_buffer,
			text: String::new(),
			window,
			modifiers: ModifiersState::default(),
		}
	}
}

struct Application {
	window_state: Option<WindowState>,
}

impl winit::application::ApplicationHandler for Application {
	fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
		if self.window_state.is_some() {
			return;
		}

		let window_attributes = Window::default_attributes().with_title("Ted");
		let window =
			Arc::new(event_loop.create_window(window_attributes).unwrap());

		self.window_state = Some(pollster::block_on(WindowState::new(window)));
	}

	fn window_event(
		&mut self,
		event_loop: &winit::event_loop::ActiveEventLoop,
		_window_id: winit::window::WindowId,
		event: WindowEvent,
	) {
		let Some(state) = &mut self.window_state else {
			return;
		};

		let WindowState {
			window,
			device,
			queue,
			surface,
			surface_config,
			font_system,
			swash_cache,
			viewport,
			atlas,
			text_renderer,
			text_buffer,
			text,
			modifiers,
			..
		} = state;

		match event {
			WindowEvent::Resized(size) => {
				surface_config.width = size.width;
				surface_config.height = size.height;
				surface.configure(device, surface_config);
				window.request_redraw();
			}
			WindowEvent::ModifiersChanged(mods) => {
				*modifiers = mods.state();
			}
			WindowEvent::KeyboardInput { event, .. } => {
				if event.state == ElementState::Pressed && !event.repeat {
					let key = event.key_without_modifiers();

					match key {
						Key::Named(NamedKey::Escape) => event_loop.exit(),
						Key::Named(NamedKey::Backspace) => {
							text.pop();
						}
						Key::Named(NamedKey::Enter) => {
							text.push('\n');
						}
						Key::Named(NamedKey::Space) => {
							text.push(' ');
						}
						Key::Character(c) => {
							let c = c.chars().next().unwrap();

							if modifiers.shift_key() {
								text.push(c.to_uppercase().next().unwrap());
							} else {
								text.push(c);
							}
						}
						_ => {}
					}

					text_buffer.set_text(
						font_system,
						text,
						&Attrs::new().family(Family::SansSerif),
						Shaping::Advanced,
					);

					text_buffer.shape_until_scroll(font_system, false);
					window.request_redraw();
				}
			}
			WindowEvent::RedrawRequested => {
				viewport.update(
					queue,
					Resolution {
						width: surface_config.width,
						height: surface_config.height,
					},
				);

				let physical_size = window.inner_size();
				let scale_factor = window.scale_factor();
				let physical_width =
					(physical_size.width as f64 * scale_factor) as f32;
				let physical_height =
					(physical_size.height as f64 * scale_factor) as f32;
				text_buffer.set_size(
					font_system,
					Some(physical_width),
					Some(physical_height),
				);

				text_renderer
					.prepare(
						device,
						queue,
						font_system,
						atlas,
						viewport,
						[TextArea {
							buffer: text_buffer,
							left: 0.0,
							top: 0.0,
							scale: 1.0,
							bounds: TextBounds {
								left: 0,
								top: 0,
								right: physical_width as i32,
								bottom: physical_height as i32,
							},
							default_color: Color::rgb(255, 255, 255),
							custom_glyphs: &[],
						}],
						swash_cache,
					)
					.unwrap();

				let frame = surface.get_current_texture().unwrap();
				let view = frame
					.texture
					.create_view(&TextureViewDescriptor::default());
				let mut encoder =
					device.create_command_encoder(&CommandEncoderDescriptor {
						label: None,
					});
				{
					let mut pass =
						encoder.begin_render_pass(&RenderPassDescriptor {
							label: None,
							color_attachments: &[Some(
								RenderPassColorAttachment {
									view: &view,
									resolve_target: None,
									depth_slice: None,
									ops: Operations {
										load: LoadOp::Clear(wgpu::Color::BLACK),
										store: wgpu::StoreOp::Store,
									},
								},
							)],
							depth_stencil_attachment: None,
							timestamp_writes: None,
							occlusion_query_set: None,
						});

					text_renderer.render(atlas, viewport, &mut pass).unwrap();
				}

				queue.submit(Some(encoder.finish()));
				frame.present();

				atlas.trim();
			}
			WindowEvent::CloseRequested => event_loop.exit(),
			_ => {}
		}
	}
}
