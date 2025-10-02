use bevy_app::{App, PostUpdate};
use bevy_camera::Camera;
use bevy_ecs::{
    schedule::IntoScheduleConfigs as _,
    system::{Commands, Query, Res},
};
use bevy_math::{Affine2, Vec2};
use bevy_sprite::Anchor;
use bevy_ui::{
    ComputedNode, DefaultUiCamera, Node, UiGlobalTransform, UiRect, UiTargetCamera, Val,
    ui_layout_system,
};
use tiny_bail::prelude::*;

use crate::{
    TooltipContent, TooltipSettings, TooltipSystems,
    context::{TooltipContext, TooltipState},
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        PostUpdate,
        (place_tooltip, run_ui_layout_system)
            .chain()
            .in_set(TooltipSystems::Placement),
    );
}

/// A target point for a tooltip entity.
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub enum TargetPoint {
    Fixed(Anchor),
    Cursor { follow: bool },
}

/// The tooltip placement configuration.
///
/// Defaults to [`Self::CURSOR_CENTERED`].
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct TooltipPlacement {
    /// The tooltip entity's anchor point.
    pub anchor_point: Anchor,
    /// The target point where the anchor point should be placed.
    pub target_point: TargetPoint,
    /// An additional horizontal offset for the tooltip entity.
    pub offset_x: Val,
    /// An additional vertical offset for the tooltip entity.
    pub offset_y: Val,
    /// Clamp the tooltip entity within the window with additional padding.
    pub clamp_padding: UiRect,
}

impl TooltipPlacement {
    /// Show the tooltip centered at the cursor.
    pub const CURSOR_CENTERED: Self = Self {
        anchor_point: Anchor::CENTER,
        target_point: TargetPoint::Cursor { follow: false },
        offset_x: Val::ZERO,
        offset_y: Val::ZERO,
        clamp_padding: UiRect::ZERO,
    };

    /// Show the tooltip at the cursor.
    pub const CURSOR: Self = Self {
        anchor_point: Anchor::TOP_LEFT,
        target_point: TargetPoint::Cursor { follow: false },
        offset_x: Val::Px(16.0),
        offset_y: Val::Px(16.0),
        clamp_padding: UiRect::ZERO,
    };

    /// Show the tooltip centered at the cursor as it moves.
    pub const FOLLOW_CURSOR_CENTERED: Self = Self {
        anchor_point: Anchor::CENTER,
        target_point: TargetPoint::Cursor { follow: true },
        offset_x: Val::ZERO,
        offset_y: Val::ZERO,
        clamp_padding: UiRect::ZERO,
    };

    /// Show the tooltip at the cursor as it moves.
    pub const FOLLOW_CURSOR: Self = Self {
        anchor_point: Anchor::TOP_LEFT,
        target_point: TargetPoint::Cursor { follow: true },
        offset_x: Val::Px(16.0),
        offset_y: Val::Px(16.0),
        clamp_padding: UiRect::ZERO,
    };
}

impl From<Anchor> for TooltipPlacement {
    fn from(value: Anchor) -> Self {
        Self {
            anchor_point: Anchor(-value.0),
            target_point: TargetPoint::Fixed(value),
            offset_x: Val::ZERO,
            offset_y: Val::ZERO,
            clamp_padding: UiRect::ZERO,
        }
    }
}

impl From<Vec2> for TooltipPlacement {
    fn from(value: Vec2) -> Self {
        Self {
            anchor_point: Anchor::TOP_LEFT,
            target_point: TargetPoint::Cursor { follow: false },
            offset_x: Val::Px(value.x),
            offset_y: Val::Px(value.y),
            clamp_padding: UiRect::ZERO,
        }
    }
}

impl Default for TooltipPlacement {
    fn default() -> Self {
        Self::CURSOR_CENTERED
    }
}

// TODO: Only run on `ShowTooltip` event OR if using target anchor + target has moved or resized.
fn place_tooltip(
    mut commands: Commands,
    ctx: Res<TooltipContext>,
    primary: Res<TooltipSettings>,
    computed_node_query: Query<&ComputedNode>,
    target_camera_query: Query<&UiTargetCamera>,
    default_ui_camera: DefaultUiCamera,
    camera_query: Query<&Camera>,
    mut node_query: Query<&mut Node>,
    mut gt_query: Query<&mut UiGlobalTransform>,
) {
    rq!(matches!(ctx.state, TooltipState::Active));
    let target_gt = rq!(gt_query.get(ctx.target));
    let target_computed = rq!(computed_node_query.get(ctx.target));
    let entity = match &ctx.tooltip.content {
        TooltipContent::Primary(_) => primary.container,
        &TooltipContent::Custom(id) => id,
    };
    let computed = r!(computed_node_query.get(entity));

    // Identify the target camera and viewport rect.
    let camera_entity = r!(target_camera_query
        .get(ctx.target)
        .map(UiTargetCamera::entity)
        .ok()
        .or(default_ui_camera.get()));
    let camera = r!(camera_query.get(camera_entity));
    let viewport = r!(camera.physical_viewport_rect());
    // Insert instead of mutate because the tooltip entity might not spawn with a `UiTargetCamera` component.
    commands
        .entity(entity)
        .insert(UiTargetCamera(camera_entity));

    let placement = &ctx.tooltip.placement;

    // Calculate target position.
    let mut pos = if let TargetPoint::Fixed(target_anchor) = placement.target_point {
        target_gt.translation - target_computed.size * target_anchor.0 * Vec2::new(-1.0, 1.0)
    } else {
        ctx.cursor_pos
    };

    // Apply tooltip anchor to target position.
    pos += computed.size * placement.anchor_point.0 * Vec2::new(-1.0, 1.0);

    // Resolve offset `Val`s.
    let size = viewport.size().as_vec2();
    let scale = camera.target_scaling_factor().unwrap_or(1.0);
    let offset_x = placement
        .offset_x
        .resolve(scale, size.x, size)
        .unwrap_or_default();
    let offset_y = placement
        .offset_y
        .resolve(scale, size.y, size)
        .unwrap_or_default();

    // Apply offset.
    pos += Vec2::new(offset_x, offset_y);

    // Resolve clamp padding `Val`s.
    let UiRect {
        left,
        right,
        top,
        bottom,
    } = placement.clamp_padding;
    let left = left.resolve(scale, size.x, size).unwrap_or_default();
    let right = right.resolve(scale, size.x, size).unwrap_or_default();
    let top = top.resolve(scale, size.x, size).unwrap_or_default();
    let bottom = bottom.resolve(scale, size.x, size).unwrap_or_default();

    // Apply clamping.
    let half_size = computed.size / 2.0;
    let mut left = half_size.x + left;
    let mut right = size.x - half_size.x - right;
    if left > right {
        let mid = (left + right) / 2.0;
        left = mid;
        right = mid;
    }
    let mut top = half_size.y + top;
    let mut bottom = size.y - half_size.y - bottom;
    if top > bottom {
        let mid = (top + bottom) / 2.0;
        top = mid;
        bottom = mid;
    }
    pos = pos.clamp(Vec2::new(left, top), Vec2::new(right, bottom));

    // Apply rounding depending on parity of size.
    if computed.size.x.round() % 2.0 < f32::EPSILON {
        pos.x = round_ties_up(pos.x);
    } else {
        pos.x = round_ties_up(pos.x + 0.5) - 0.5;
    }
    if computed.size.y.round() % 2.0 < f32::EPSILON {
        pos.y = round_ties_up(pos.y);
    } else {
        pos.y = round_ties_up(pos.y + 0.5) - 0.5;
    }

    // Set position via `UiGlobalTransform`.
    // This system has to run after `UiSystem::Layout` so that its size is calculated
    // from the updated text. However, that means that `Node` positioning will be
    // delayed by 1 frame. As a workaround, update the `UiGlobalTransform` directly as well.
    let mut gt = r!(gt_query.get_mut(entity));
    *gt = {
        let mut gt = Affine2::from(*gt);
        gt.translation = pos;
        gt.into()
    };

    // Set position via `Node`.
    pos -= half_size;
    let mut node = r!(node_query.get_mut(entity));
    node.left = Val::Px(pos.x);
    node.top = Val::Px(pos.y);
}

/// Taken from `bevy_ui`, used in `ui_layout_system`.
fn round_ties_up(value: f32) -> f32 {
    if value.fract() != -0.5 {
        value.round()
    } else {
        value.ceil()
    }
}

// FIXME: This is a lazy workaround for `UiTransform` propagation being coupled to
//        `ui_layout_system` in Bevy 0.17. It's inefficient but it works.
fn run_ui_layout_system(world: &mut bevy_ecs::world::World) {
    let _ = world.run_system_cached(ui_layout_system);
}
