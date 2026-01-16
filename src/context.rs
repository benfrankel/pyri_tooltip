use bevy_app::{App, PreUpdate};
use bevy_camera::{Camera, RenderTarget, visibility::Visibility};
#[cfg(feature = "bevy_reflect")]
use bevy_ecs::reflect::ReflectResource;
use bevy_ecs::{
    entity::Entity,
    message::{Message, MessageReader, MessageWriter},
    query::With,
    resource::Resource,
    schedule::{IntoScheduleConfigs as _, common_conditions::on_message},
    system::{Query, Res, ResMut},
};
use bevy_math::Vec2;
use bevy_time::Time;
use bevy_ui::{Interaction, UiStack};
use bevy_window::{PrimaryWindow, Window, WindowRef};
use tiny_bail::prelude::*;

use crate::{
    Tooltip, TooltipContent, TooltipSettings, TooltipSystems, placement::TargetPoint,
    rich_text::RichText,
};

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<TooltipContext>();
    app.add_message::<HideTooltip>();
    app.add_message::<ShowTooltip>();
    app.add_systems(
        PreUpdate,
        (
            update_tooltip_context,
            hide_tooltip.run_if(on_message::<HideTooltip>),
            show_tooltip.run_if(on_message::<ShowTooltip>),
        )
            .chain()
            .in_set(TooltipSystems::Content),
    );
}

/// A [`Resource`] that contains the current values in use by the tooltip system.
#[derive(Resource, Clone, Debug)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(bevy_reflect::Reflect),
    reflect(Resource)
)]
pub(crate) struct TooltipContext {
    /// The current state of the tooltip system.
    pub(crate) state: TooltipState,
    /// The current or previous target entity being interacted with.
    pub(crate) target: Entity,
    /// The remaining duration of the current activation delay or transfer timeout (in milliseconds).
    timer: u16,
    /// The current cursor position or activation point.
    pub(crate) cursor_pos: Vec2,
    /// The current tooltip parameters.
    pub(crate) tooltip: Tooltip,
}

impl Default for TooltipContext {
    fn default() -> Self {
        Self {
            state: TooltipState::Inactive,
            target: Entity::PLACEHOLDER,
            timer: 0,
            cursor_pos: Vec2::ZERO,
            tooltip: Tooltip::cursor(Entity::PLACEHOLDER),
        }
    }
}

fn update_tooltip_context(
    mut ctx: ResMut<TooltipContext>,
    mut hide_tooltip: MessageWriter<HideTooltip>,
    mut show_tooltip: MessageWriter<ShowTooltip>,
    primary: Res<TooltipSettings>,
    time: Res<Time>,
    ui_stack: Res<UiStack>,
    primary_window_query: Query<Entity, With<PrimaryWindow>>,
    window_query: Query<&Window>,
    camera_query: Query<&RenderTarget, With<Camera>>,
    interaction_query: Query<(&Tooltip, &Interaction)>,
) {
    let old_active = matches!(ctx.state, TooltipState::Active);
    let old_target = ctx.target;
    let old_entity = match ctx.tooltip.content {
        TooltipContent::Primary(_) => primary.container,
        TooltipContent::Custom(id) => id,
    };

    // TODO: Reconsider whether this is the right way to detect cursor movement.
    // Detect cursor movement.
    for camera_target in &camera_query {
        let RenderTarget::Window(window) = camera_target else {
            continue;
        };

        let window = match window {
            WindowRef::Primary => cq!(primary_window_query.single()),
            WindowRef::Entity(id) => *id,
        };
        let window = c!(window_query.get(window));

        cq!(window.focused);
        let cursor_pos = cq!(window.cursor_position());

        // Reset activation delay on cursor move.
        if ctx.cursor_pos != cursor_pos
            && matches!(ctx.state, TooltipState::Delayed)
            && ctx.tooltip.activation.reset_delay_on_cursor_move
        {
            ctx.timer = ctx.tooltip.activation.delay;
        }

        // Dismiss tooltip if cursor has left the activation radius.
        if matches!(ctx.state, TooltipState::Active)
            && ctx.cursor_pos.distance_squared(cursor_pos) > ctx.tooltip.dismissal.on_distance
        {
            ctx.state = TooltipState::Dismissed;
        }

        // Update cursor position.
        if !matches!(ctx.state, TooltipState::Active)
            || matches!(
                ctx.tooltip.placement.target_point,
                TargetPoint::Cursor { follow: true }
            )
        {
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
    let mut found_target = false;
    for &entity in ui_stack.uinodes.iter().rev() {
        let (tooltip, interaction) = cq!(interaction_query.get(entity));
        match interaction {
            Interaction::Pressed if tooltip.dismissal.on_click => {
                ctx.target = entity;
                ctx.state = TooltipState::Dismissed;
                ctx.tooltip.transfer = tooltip.transfer;
                found_target = true;
                break;
            }
            Interaction::None => continue,
            _ => (),
        };

        // Still hovering the same target entity.
        if ctx.target == entity && !matches!(ctx.state, TooltipState::Inactive) {
            ctx.tooltip = tooltip.clone();
            ctx.tooltip.dismissal.on_distance *= ctx.tooltip.dismissal.on_distance;
            found_target = true;
            break;
        }

        // Switch to the new target entity.
        ctx.state = if tooltip.activation.delay == 0
            || (matches!(ctx.state, TooltipState::Inactive)
                && ctx.timer > 0
                && ctx.tooltip.transfer.layer >= tooltip.transfer.layer
                && (matches!((ctx.tooltip.transfer.group, tooltip.transfer.group), (Some(x), Some(y)) if x == y)
                    || ctx.target == entity))
        {
            TooltipState::Active
        } else {
            TooltipState::Delayed
        };
        ctx.target = entity;
        ctx.timer = tooltip.activation.delay;
        ctx.tooltip = tooltip.clone();
        ctx.tooltip.dismissal.on_distance *= ctx.tooltip.dismissal.on_distance;
        found_target = true;
        break;
    }

    // There is no longer a target entity.
    if !found_target && !matches!(ctx.state, TooltipState::Inactive) {
        ctx.timer =
            if matches!(ctx.state, TooltipState::Active) || !ctx.tooltip.transfer.from_active {
                ctx.tooltip.transfer.timeout
            } else {
                0
            };
        ctx.state = TooltipState::Inactive;
    }

    // Update tooltip if it has a target, or was activated, dismissed, or changed targets.
    let new_active = matches!(ctx.state, TooltipState::Active);
    if old_active != new_active || old_target != ctx.target || found_target {
        if old_active {
            hide_tooltip.write(HideTooltip { entity: old_entity });
        }
        if new_active {
            show_tooltip.write(ShowTooltip);
        }
    }
}

/// The current state of the tooltip system.
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub(crate) enum TooltipState {
    /// There is no target entity being interacted with, and no active tooltip.
    Inactive,
    /// A target entity is being hovered, but its tooltip is not active yet.
    Delayed,
    /// A target entity is being hovered, and its tooltip is active.
    Active,
    /// A target entity is being interacted with, but its tooltip has been dismissed.
    Dismissed,
}

/// A message written when a tooltip should be hidden.
#[derive(Message)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
struct HideTooltip {
    entity: Entity,
}

fn hide_tooltip(
    mut hide_tooltip: MessageReader<HideTooltip>,
    mut visibility_query: Query<&mut Visibility>,
) {
    for event in hide_tooltip.read() {
        *cq!(visibility_query.get_mut(event.entity)) = Visibility::Hidden;
    }
}

/// A message written when a tooltip should be shown.
#[derive(Message)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
struct ShowTooltip;

fn show_tooltip(
    mut ctx: ResMut<TooltipContext>,
    primary: Res<TooltipSettings>,
    mut text_query: Query<&mut RichText>,
    mut visibility_query: Query<&mut Visibility>,
) {
    let entity = match ctx.tooltip.content {
        TooltipContent::Primary(ref mut text) => {
            if let Ok(mut primary_text) = text_query.get_mut(primary.text) {
                *primary_text = core::mem::take(text);
            }
            primary.container
        }
        TooltipContent::Custom(id) => id,
    };

    *r!(visibility_query.get_mut(entity)) = Visibility::Visible;
}
