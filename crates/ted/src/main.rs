use bevy::{
	app::{App, Startup, Update},
	asset::{AssetServer, Assets},
	color::Color,
	input::ButtonInput,
	math::Vec3,
	prelude::{
		Camera2d, Camera3d, Commands, Component, KeyCode, Query, Res, ResMut,
		Text, Transform,
	},
	sprite::ColorMaterial,
	text::{Text2d, TextColor, TextFont},
	time::Time,
	ui::{GridPlacement, Node, PositionType, Val},
	window::Window,
	DefaultPlugins,
};

// Text editor with smooth camera movement
#[derive(Component)]
pub struct RigidBody {
	pub velocity: Vec3,
}

#[derive(Component)]
pub struct TextEditor {
	pub row: usize,
	pub column: usize,
	pub camera_scale_velocity: Vec3,
}

fn main() {
	App::new()
		.add_plugins((DefaultPlugins,))
		.add_systems(Startup, (setup, setup_camera))
		.add_systems(Update, keyboard_input)
		.run();
}

fn keyboard_input(
	time: Res<Time>,
	keyboard_input: Res<ButtonInput<KeyCode>>,
	mut query: Query<(&mut TextEditor, &mut RigidBody, &mut Transform)>,
	mut text: Query<&mut Text2d>,
	mut window: Query<&Window>,
) {
	let (mut text_editor, mut rigid_body, mut camera_transform) =
		query.iter_mut().next().unwrap();
	let text = text.iter().next().unwrap();

	let max_line_len =
		text.0.lines().map(|line| line.len()).max().unwrap_or(0) as f32;
	let w = window.iter().next().unwrap().width();
	let mut target_scale = w / 3.0 / (max_line_len * 0.75);

	let mut cursor_pos = Vec3::new(0.0, 0.0, 0.0);
	let cursor_row = text_editor.row;
	let cursor_col = text_editor.column;
	let cursor_offset = 0.0;
	let mut cursor_row_offset = 0.0;

	for (i, line) in text.0.lines().enumerate() {
		if i == cursor_row {
			cursor_pos.y = -((i as f32) + 0.5) * 18.0;
			cursor_pos.x = 0.0;
			for (j, c) in line.chars().enumerate() {
				if j == cursor_col {
					break;
				}
				cursor_pos.x += 18.0;
			}
			break;
		}
		cursor_row_offset += 18.0;
	}

	let mut target = cursor_pos;

	if target_scale > 3.0 {
		target_scale = 3.0;
	} else {
		let mut offset = cursor_pos.x - w / 3.0 / camera_transform.scale.x;
		if offset < 0.0 {
			offset = 0.0;
		}
		target = Vec3::new(
			w / 3.0 / camera_transform.scale.x + offset,
			cursor_pos.y,
			0.0,
		);
	}

	rigid_body.velocity = (target - camera_transform.translation)
		* Vec3::new(2.0, 2.0, 2.0);
	text_editor.camera_scale_velocity =
		(target_scale - camera_transform.scale) * Vec3::new(2.0, 2.0, 2.0);
	
	camera_transform.translation += rigid_body.velocity * time.delta_secs();
	camera_transform.scale += text_editor.camera_scale_velocity * time.delta_secs();

	if keyboard_input.pressed(KeyCode::ArrowLeft) {
		if text_editor.column > 0 {
			text_editor.column -= 1;
		}
	} else if keyboard_input.pressed(KeyCode::ArrowRight) {
		if text_editor.column
			< text.0.lines().nth(text_editor.row).unwrap().len()
		{
			text_editor.column += 1;
		}
	} else if keyboard_input.pressed(KeyCode::ArrowUp) {
		if text_editor.row > 0 {
			text_editor.row -= 1;
		}
	} else if keyboard_input.pressed(KeyCode::ArrowDown) {
		if text_editor.row < text.0.lines().count() - 1 {
			text_editor.row += 1;
		}
	}

	// Reset the velocity if the key is released
	if !keyboard_input.pressed(KeyCode::ArrowLeft)
		&& !keyboard_input.pressed(KeyCode::ArrowRight)
		&& !keyboard_input.pressed(KeyCode::ArrowUp)
		&& !keyboard_input.pressed(KeyCode::ArrowDown)
	{
		rigid_body.velocity = Vec3::new(0.0, 0.0, 0.0);
	}
}

fn setup_camera(mut query: Query<(&mut Camera2d, &mut Transform)>) {
	if let Some((mut camera, mut transform)) = query.iter_mut().next() {
		transform.scale = Vec3::new(3.0, 3.0, 3.0);
	}
}

fn setup(asset_server: Res<AssetServer>, mut commands: Commands) {
	let text_font = TextFont {
		font: asset_server
			.load("fonts/ComicShannsMonoNerdFontMono-Regular.otf"),
		font_size: 64.0,
		..Default::default()
	};

	commands.spawn((
		TextEditor {
			row: 0,
			column: 0,
			camera_scale_velocity: Vec3::new(0.0, 0.0, 0.0),
		},
		Camera2d {
			..Default::default()
		},
		RigidBody {
			velocity: Vec3::new(0.0, 0.0, 0.0),
		},
	));
	commands.spawn((
		Text2d::new(include_str!("main.rs")),
		Node {
			..Default::default()
		},
		TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
		text_font,
	));
}
