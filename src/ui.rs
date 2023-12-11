use bevy::{prelude::*, render::view::RenderLayers, sprite::MaterialMesh2dBundle};
use bevy_persistent::Persistent;

use crate::{config::GameOptions, load::GameAssets, GameState};

pub const UI_LAYER: RenderLayers = RenderLayers::layer(1);
const MENU_WIDTH: Val = Val::Px(300.);
const MENU_ITEM_HEIGHT: Val = Val::Px(40.);
const MENU_ITEM_GAP: Val = Val::Px(10.);

// ······
// Plugin
// ······

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(UIStyle::default())
            .add_systems(OnEnter(GameState::Loading), init_ui)
            .add_systems(
                PostUpdate,
                change_style.run_if(resource_changed::<Persistent<GameOptions>>()),
            );
    }
}

// ·········
// Resources
// ·········

#[derive(Resource, Default)]
pub struct UIStyle {
    title: TextStyle,
    pub text: TextStyle,
    button_text: TextStyle,

    button: Style,
    button_bg: BackgroundColor,
}

// ··········
// Components
// ··········

#[derive(Component)]
pub struct UiCam;

#[derive(Component)]
pub struct UiNode;

#[derive(Component)]
pub struct UiNone;

// ·······
// Systems
// ·······

pub fn init_ui(
    mut cmd: Commands,
    opts: Res<Persistent<GameOptions>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // UI Camera
    cmd.spawn((
        Camera2dBundle {
            camera: Camera {
                order: -10,
                ..default()
            },
            ..default()
        },
        UI_LAYER,
        UiCam,
    ));

    // Background
    cmd.spawn((
        MaterialMesh2dBundle {
            mesh: meshes.add(Mesh::from(shape::Quad::default())).into(),
            transform: Transform::from_xyz(0., 0., -10.).with_scale(Vec3::new(1080., 720., 1.)),
            material: materials.add(ColorMaterial::from(opts.color.dark)),
            ..default()
        },
        UI_LAYER,
    ));

    // Main node
    cmd.spawn((
        NodeBundle {
            style: Style {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(12.),
                ..default()
            },
            ..default()
        },
        UI_LAYER,
        UiNode,
    ));
}

fn change_style(
    mut style: ResMut<UIStyle>,
    opts: Res<Persistent<GameOptions>>,
    assets: Res<GameAssets>,
) {
    style.title = TextStyle {
        font: assets.font.clone(),
        font_size: opts.font_size.title,
        color: opts.color.mid,
    };

    style.text = TextStyle {
        font: assets.font.clone(),
        font_size: opts.font_size.text,
        color: opts.color.mid,
    };

    style.button_text = TextStyle {
        font: assets.font.clone(),
        font_size: opts.font_size.button_text,
        color: opts.color.dark,
    };

    style.button = Style {
        width: MENU_WIDTH,
        height: MENU_ITEM_HEIGHT,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    };

    style.button_bg = opts.color.light.into();
}

// ·····
// Extra
// ·····

// Text

pub struct UIText<'a, T: Component> {
    text: TextBundle,
    style: &'a UIStyle,
    action: Option<T>,
}

impl<'a, T: Component> UIText<'a, T> {
    pub fn new(style: &'a UIStyle, text: &str, action: Option<T>) -> Self {
        Self {
            text: TextBundle::from_section(text, style.text.clone()),
            style,
            action,
        }
    }

    pub fn with_title(mut self) -> Self {
        self.text.text.sections[0].style = self.style.title.clone();
        self
    }

    pub fn with_style(mut self, style: Style) -> Self {
        self.text.style = style;
        self
    }

    pub fn add(self, parent: &mut ChildBuilder) {
        if let Some(action) = self.action {
            parent.spawn((self.text, action, UI_LAYER));
        } else {
            parent.spawn((self.text, UI_LAYER));
        }
    }
}

impl<'a> UIText<'a, UiNone> {
    pub fn simple(style: &'a UIStyle, text: &str) -> Self {
        Self {
            text: TextBundle::from_section(text, style.text.clone()),
            style,
            action: None,
        }
    }
}

// Button

pub struct UIButton<T: Component> {
    button: ButtonBundle,
    text: TextBundle,
    action: Option<T>,
}

impl<T: Component> UIButton<T> {
    pub fn new(style: &UIStyle, text: &str, action: Option<T>) -> Self {
        Self {
            button: ButtonBundle {
                style: style.button.clone(),
                background_color: style.button_bg,
                ..default()
            },
            text: TextBundle::from_section(text, style.button_text.clone()),
            action,
        }
    }

    pub fn with_width(mut self, width: Val) -> Self {
        self.button.style.width = width;
        self
    }

    pub fn with_font_scale(mut self, scale: f32) -> Self {
        self.text.text.sections[0].style.font_size *= scale;
        self
    }

    pub fn add(self, parent: &mut ChildBuilder) {
        if let Some(action) = self.action {
            parent.spawn((self.button, action, UI_LAYER))
        } else {
            parent.spawn((self.button, UI_LAYER))
        }
        .with_children(|button| {
            button.spawn((self.text, UI_LAYER));
        });
    }
}

// Option row (label text + widget)

pub struct UIOption<'a> {
    row: NodeBundle,
    label: UIText<'a, UiNone>,
}

impl<'a> UIOption<'a> {
    pub fn new(style: &'a UIStyle, label: &str) -> Self {
        Self {
            row: NodeBundle {
                style: Style {
                    width: MENU_WIDTH,
                    column_gap: MENU_ITEM_GAP,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                ..default()
            },
            label: UIText::new(style, &snake_to_upper(label), None).with_style(Style {
                flex_grow: 1.,
                ..default()
            }),
        }
    }

    pub fn add(self, parent: &mut ChildBuilder, children: impl FnOnce(&mut ChildBuilder)) {
        parent.spawn((self.row, UI_LAYER)).with_children(|row| {
            self.label.add(row);
            children(row);
        });
    }
}

pub fn snake_to_upper(text: &str) -> String {
    text.chars()
        .enumerate()
        .map(|(i, c)| {
            if i == 0 {
                c.to_uppercase().next().unwrap_or(c)
            } else if c == '_' {
                ' '
            } else {
                c
            }
        })
        .collect::<String>()
}
