//! TODO

/// TODO
pub mod prelude {
    pub use super::{
        PrimaryTooltip, Tooltip, TooltipActivation, TooltipEntity, TooltipPlacement, TooltipPlugin,
        TooltipTransfer,
    };
}

use bevy_app::{Plugin, PreUpdate};
use bevy_core::Name;
#[cfg(feature = "bevy_reflect")]
use bevy_ecs::reflect::{ReflectComponent, ReflectResource};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::With,
    system::{Query, Res, ResMut, Resource},
    world::World,
};
use bevy_hierarchy::BuildWorldChildren as _;
use bevy_math::Vec2;
use bevy_render::{
    camera::{Camera, RenderTarget},
    view::Visibility,
};
use bevy_sprite::Anchor;
use bevy_text::{Text, TextSection, TextStyle};
use bevy_time::Time;
use bevy_transform::components::GlobalTransform;
use bevy_ui::{
    node_bundles::{NodeBundle, TextBundle},
    Interaction, PositionType, Style, UiRect, UiStack, Val, ZIndex,
};
use bevy_window::{PrimaryWindow, Window, WindowRef};
use tiny_bail::prelude::*;

/// TODO
#[derive(Default)]
pub struct TooltipPlugin {
    /// Set a custom entity for [`PrimaryTooltip::container`], or spawn a default entity if `None`.
    pub container_entity: Option<Entity>,
    /// Set a custom entity for [`PrimaryTooltip::text`], or spawn a default entity if `None`.
    pub text_entity: Option<Entity>,
}

impl Plugin for TooltipPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.register_type::<PrimaryTooltip>();
        let primary_tooltip =
            PrimaryTooltip::new(app.world_mut(), self.container_entity, self.text_entity);
        app.insert_resource(primary_tooltip);

        app.register_type::<TooltipContext>();
        app.init_resource::<TooltipContext>();
        // TODO: Make sure this runs after `Interaction` is updated.
        app.add_systems(PreUpdate, update_tooltip_context);

        app.register_type::<Tooltip>();
    }
}

/// TODO
#[derive(Resource, Copy, Clone, Debug)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(bevy_reflect::Reflect),
    reflect(Resource)
)]
pub struct PrimaryTooltip {
    /// The [`Entity`] ID of the UI node to be used as the primary tooltip.
    pub container: Entity,
    /// The [`Entity`] ID of the UI node to be used as the primary tooltip's text.
    pub text: Entity,
}

impl PrimaryTooltip {
    fn new(world: &mut World, container: Option<Entity>, text: Option<Entity>) -> Self {
        let container = container.unwrap_or_else(|| {
            world
                .spawn((
                    Name::new("PrimaryTooltip"),
                    NodeBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            max_width: Val::Vw(40.0),
                            padding: UiRect::all(Val::Px(8.0)),
                            ..Default::default()
                        },
                        visibility: Visibility::Hidden,
                        z_index: ZIndex::Global(999),
                        ..Default::default()
                    },
                ))
                .id()
        });

        let text = text.unwrap_or_else(|| {
            world
                .spawn((Name::new("Text"), TextBundle::default()))
                .set_parent(container)
                .id()
        });

        Self { container, text }
    }
}

fn update_tooltip_context(
    mut ctx: ResMut<TooltipContext>,
    time: Res<Time>,
    ui_stack: Res<UiStack>,
    primary_window_query: Query<Entity, With<PrimaryWindow>>,
    window_query: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    interaction_query: Query<(&Tooltip, &Interaction)>,
) {
    // TODO: Is this needed? Send an event? Show / hide the tooltip?
    let _old_state = ctx.state;

    // Detect cursor movement.
    for (camera, camera_gt) in &camera_query {
        let RenderTarget::Window(window) = camera.target else {
            continue;
        };
        let window = match window {
            WindowRef::Primary => cq!(primary_window_query.get_single()),
            WindowRef::Entity(id) => id,
        };
        let window = c!(window_query.get(window));
        cq!(window.focused);
        let cursor_pos = cq!(window
            .cursor_position()
            .and_then(|cursor| camera.viewport_to_world_2d(camera_gt, cursor)));

        // Reset activation delay on cursor move.
        if ctx.cursor_pos != cursor_pos
            && matches!(ctx.state, TooltipState::Delayed)
            && ctx.activation.reset_delay_on_cursor_move
        {
            ctx.timer = ctx.activation.delay;
        }

        // Dismiss tooltip if cursor has left the activation radius.
        if matches!(ctx.state, TooltipState::Active)
            && ctx.cursor_pos.distance_squared(cursor_pos) > ctx.activation.radius
        {
            ctx.state = TooltipState::Dismissed;
        }

        // Update cursor position.
        if !matches!(ctx.state, TooltipState::Active) {
            ctx.cursor_pos = cursor_pos;
        }

        break;
    }

    // Tick timer for transfer timeout / activation delay.
    if matches!(ctx.state, TooltipState::Inactive | TooltipState::Delayed) {
        ctx.timer = ctx.timer.saturating_sub(time.delta().as_millis() as u16);
        if matches!(ctx.state, TooltipState::Delayed) && ctx.timer == 0 {
            ctx.state = TooltipState::Active;
        }
    }

    // Find the highest entity in the `UiStack` that has a tooltip and is being interacted with.
    for &entity in ui_stack.uinodes.iter().rev() {
        let (tooltip, interaction) = cq!(interaction_query.get(entity));
        match interaction {
            Interaction::Pressed => {
                ctx.target = entity;
                ctx.state = TooltipState::Dismissed;
                ctx.transfer = tooltip.transfer;
                return;
            }
            Interaction::Hovered => (),
            Interaction::None => continue,
        };
        rq!(matches!(ctx.state, TooltipState::Inactive) || ctx.target != entity);

        // Switch to the new target entity.
        ctx.target = entity;
        ctx.state = if tooltip.activation.delay == 0
            || (matches!(ctx.state, TooltipState::Inactive)
                && ctx.timer > 0
                && ctx.transfer.layer >= tooltip.transfer.layer
                && (matches!((ctx.transfer.group, tooltip.transfer.group), (Some(x), Some(y)) if x == y)
                    || ctx.target == entity))
        {
            TooltipState::Active
        } else {
            TooltipState::Delayed
        };
        ctx.timer = tooltip.activation.delay;
        ctx.activation = tooltip.activation;
        ctx.activation.radius *= ctx.activation.radius;
        ctx.transfer = tooltip.transfer;

        return;
    }

    // There is no target entity.
    if !matches!(ctx.state, TooltipState::Inactive) {
        ctx.timer = if matches!(ctx.state, TooltipState::Active) || !ctx.transfer.from_active {
            ctx.transfer.timeout
        } else {
            0
        };
        ctx.state = TooltipState::Inactive;
    }
}

// TODO: Animation, wedge (like a speech bubble), easier content customization / icons.
/// TODO
#[derive(Component)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(bevy_reflect::Reflect),
    reflect(Component)
)]
pub struct Tooltip {
    /// The conditions for activating and dismissing the tooltip.
    pub activation: TooltipActivation,
    /// The conditions for skipping the next tooltip's activation delay.
    pub transfer: TooltipTransfer,
    /// How the position of the tooltip entity should be determined.
    pub placement: TooltipPlacement,
    /// The entity to display as the tooltip.
    pub entity: TooltipEntity,
}

impl Tooltip {
    /// Use the given tooltip entity and default behavior.
    fn new(entity: TooltipEntity) -> Self {
        Self {
            activation: Default::default(),
            transfer: Default::default(),
            placement: Default::default(),
            entity,
        }
    }

    /// Use the primary tooltip entity with a single [`TextSection`] and default behavior.
    pub fn from_section(value: impl Into<String>, style: TextStyle) -> Self {
        Self::new(TooltipEntity::Primary(Text::from_section(value, style)))
    }

    /// Use the primary tooltip entity with a list of [`TextSection`]s and default behavior.
    pub fn from_sections(sections: impl IntoIterator<Item = TextSection>) -> Self {
        Self::new(TooltipEntity::Primary(Text::from_sections(sections)))
    }

    /// Use the primary tooltip entity with a given [`Text`] and default behavior.
    pub fn from_text(text: impl Into<Text>) -> Self {
        Self::new(TooltipEntity::Primary(text.into()))
    }

    /// Use a custom tooltip entity and default behavior.
    pub fn custom(entity: Entity) -> Self {
        Self::new(TooltipEntity::Custom(entity))
    }

    /// Set a custom [`TooltipActivation`].
    pub fn with_activation(mut self, activation: TooltipActivation) -> Self {
        self.activation = activation;
        self
    }

    /// Set a custom [`TooltipTransfer`].
    pub fn with_transfer(mut self, transfer: TooltipTransfer) -> Self {
        self.transfer = transfer;
        self
    }

    /// Set a custom [`TooltipPlacement`].
    pub fn with_placement(mut self, placement: TooltipPlacement) -> Self {
        self.placement = placement;
        self
    }
}

/// TODO
#[derive(Resource, Clone, Debug)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(bevy_reflect::Reflect),
    reflect(Resource)
)]
struct TooltipContext {
    /// The current state of the tooltip system.
    state: TooltipState,
    /// The current or previous target entity being interacted with.
    target: Entity,
    /// The remaining duration of the current activation delay or transfer timeout (in milliseconds).
    timer: u16,
    /// The current activation conditions.
    activation: TooltipActivation,
    /// The current transfer conditions.
    transfer: TooltipTransfer,
    /// The current cursor position or activation point.
    cursor_pos: Vec2,
}

impl Default for TooltipContext {
    fn default() -> Self {
        Self {
            state: TooltipState::Inactive,
            target: Entity::PLACEHOLDER,
            timer: 0,
            activation: TooltipActivation::DEFAULT,
            transfer: TooltipTransfer::DEFAULT,
            cursor_pos: Vec2::ZERO,
        }
    }
}

/// TODO
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
enum TooltipState {
    /// There is no target entity being interacted with, and no active tooltip.
    Inactive,
    /// A target entity is being hovered, but its tooltip is not active yet.
    Delayed,
    /// A target entity is being hovered, and its tooltip is active.
    Active,
    /// A target entity is being interacted with, but its tooltip has been dismissed.
    Dismissed,
}

/// TODO
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct TooltipActivation {
    /// The hover duration before the tooltip will activate (in milliseconds).
    pub delay: u16,
    /// Whether to reset the activation delay timer whenever the cursor moves.
    pub reset_delay_on_cursor_move: bool,
    /// The radius around the activation point beyond which the tooltip will be dismissed.
    pub radius: f32,
}

impl TooltipActivation {
    /// The default `TooltipActivation`.
    pub const DEFAULT: Self = Self {
        delay: 100,
        reset_delay_on_cursor_move: false,
        radius: f32::INFINITY,
    };
}

impl Default for TooltipActivation {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// TODO
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct TooltipTransfer {
    /// Only transfer to elements in the same group, or to self if `None`.
    pub group: Option<i8>,
    /// Only transfer to elements in the same layer or lower.
    pub layer: i8,
    /// Only transfer within this duration after the cursor moves away from the old target (in milliseconds).
    pub timeout: u16,
    /// Only transfer if the old tooltip was active.
    pub from_active: bool,
}

impl TooltipTransfer {
    /// The default `TooltipTransfer`.
    pub const DEFAULT: Self = Self {
        group: None,
        layer: 0,
        timeout: 0,
        from_active: true,
    };
}

impl Default for TooltipTransfer {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// TODO
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct TooltipPlacement {
    /// The anchor point on the tooltip entity.
    pub anchor: Anchor,
    /// The target position expressed as an anchor point on the target entity, or `None` to use the cursor position instead.
    pub target: Option<Anchor>,
    /// An additional horizontal offset for the tooltip entity.
    pub offset_x: Val,
    /// An additional vertical offset for the tooltip entity.
    pub offset_y: Val,
    /// Clamp the tooltip entity within the window with additional padding.
    pub clamp_padding: UiRect,
}

impl TooltipPlacement {
    /// The default `TooltipPlacement`.
    pub const DEFAULT: Self = Self {
        anchor: Anchor::TopLeft,
        target: None,
        offset_x: Val::Px(16.0),
        offset_y: Val::Px(16.0),
        clamp_padding: UiRect::all(Val::ZERO),
    };
}

impl Default for TooltipPlacement {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// TODO
#[derive(Clone, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub enum TooltipEntity {
    /// Use the primary tooltip entity with custom [`Text`].
    Primary(Text),
    /// Use a fully custom entity as the tooltip.
    Custom(Entity),
}
