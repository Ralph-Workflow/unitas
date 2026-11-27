use std::collections::HashMap;

use unitas_asset::scene::{
    CanvasComponent, CanvasMatchMode, CanvasRenderMode, CanvasScaleMode, RectTransformComponent,
    UiBinding, UiButtonComponent, UiElementComponent,
};
use unitas_core::EntityId;
use unitas_render::ui_pass::{FOCUS_INDICATOR_COLOR, focus_border_rects};

use crate::input::{
    GAMEPAD_AXIS_LEFT_X, GAMEPAD_AXIS_LEFT_Y, GAMEPAD_BUTTON_DPAD_DOWN, GAMEPAD_BUTTON_DPAD_LEFT,
    GAMEPAD_BUTTON_DPAD_RIGHT, GAMEPAD_BUTTON_DPAD_UP, GAMEPAD_BUTTON_SOUTH, InputState,
    KEY_DOWN_ARROW, KEY_LEFT_ARROW, KEY_LEFT_SHIFT, KEY_RETURN, KEY_RIGHT_ARROW, KEY_RIGHT_SHIFT,
    KEY_SPACE, KEY_TAB, KEY_UP_ARROW, STATE_PRESSED,
};
use crate::ui::UiTextItem;

pub use unitas_render::ui_pass::SpriteRef;

const MAX_HIERARCHY_DEPTH: usize = 64;
const FOCUS_BORDER_THICKNESS: f32 = 2.0;
const STICK_NAVIGATION_THRESHOLD: f64 = 0.5;

#[derive(Clone, Debug, PartialEq)]
pub struct UiDrawItem {
    pub rect_px: [f32; 4],
    pub color: [f32; 4],
    pub sprite: Option<SpriteRef>,
    pub text: Option<UiTextItem>,
}

impl unitas_render::ui_pass::UiElement for UiDrawItem {
    fn rect(&self) -> [f32; 4] {
        self.rect_px
    }

    fn color(&self) -> [f32; 4] {
        self.color
    }

    fn sprite(&self) -> Option<&SpriteRef> {
        self.sprite.as_ref()
    }

    fn text(&self) -> Option<unitas_render::ui_pass::UiText<'_>> {
        self.text
            .as_ref()
            .map(|item| unitas_render::ui_pass::UiText {
                text: &item.text,
                x: item.x,
                y: item.y,
                size_px: item.size_px,
                color: item.color,
            })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasRect {
    pub min: [f32; 2],
    pub size: [f32; 2],
}

/// The runtime state of a `Text` component: its content, font size and colour.
#[derive(Clone, Debug, PartialEq)]
pub struct TextOverride {
    pub text: String,
    pub font_size: f32,
    pub color: [f32; 4],
}

pub fn resolve_canvas_rect(rect: &RectTransformComponent, parent: CanvasRect) -> CanvasRect {
    let anchor_size = [
        (rect.anchor_max[0] - rect.anchor_min[0]) * parent.size[0],
        (rect.anchor_max[1] - rect.anchor_min[1]) * parent.size[1],
    ];
    let size = [
        anchor_size[0] + rect.size_delta[0],
        anchor_size[1] + rect.size_delta[1],
    ];
    let pivot = [
        rect.anchor_min[0] * parent.size[0]
            + anchor_size[0] * rect.pivot[0]
            + rect.anchored_position[0],
        rect.anchor_min[1] * parent.size[1]
            + anchor_size[1] * rect.pivot[1]
            + rect.anchored_position[1],
    ];
    CanvasRect {
        min: [
            parent.min[0] + pivot[0] - rect.pivot[0] * size[0],
            parent.min[1] + pivot[1] - rect.pivot[1] * size[1],
        ],
        size,
    }
}

pub fn canvas_scale_factor(canvas: &CanvasComponent, screen_size: [f32; 2]) -> f32 {
    let scaler = match canvas.scaler.as_ref() {
        Some(scaler) => scaler,
        None => return 1.0,
    };
    if scaler.ui_scale_mode != CanvasScaleMode::ScaleWithScreenSize {
        return 1.0;
    }
    let reference = scaler.reference_resolution;
    if reference[0] <= 0.0 || reference[1] <= 0.0 {
        return 1.0;
    }
    let width_ratio = screen_size[0] / reference[0];
    let height_ratio = screen_size[1] / reference[1];
    match scaler.screen_match_mode {
        CanvasMatchMode::MatchWidthOrHeight => {
            if scaler.match_width_or_height < 0.5 {
                width_ratio
            } else if scaler.match_width_or_height > 0.5 {
                height_ratio
            } else {
                (width_ratio * height_ratio).sqrt()
            }
        }
        CanvasMatchMode::Shrink => width_ratio.min(height_ratio),
        CanvasMatchMode::Expand => width_ratio.max(height_ratio),
    }
}

pub struct CanvasState {
    binding: UiBinding,
    rects: HashMap<EntityId, RectTransformComponent>,
    focused: Option<EntityId>,
    focus_axis_prev: [f64; 2],
    pending_clicks: Vec<u64>,
    element_enabled: HashMap<EntityId, bool>,
    button_interactable: HashMap<EntityId, bool>,
}

impl CanvasState {
    pub fn new(binding: UiBinding) -> Self {
        let rects = binding
            .rect_transforms
            .iter()
            .map(|rect| (rect.entity, rect.clone()))
            .collect();
        Self {
            binding,
            rects,
            focused: None,
            focus_axis_prev: [0.0; 2],
            pending_clicks: Vec::new(),
            element_enabled: HashMap::new(),
            button_interactable: HashMap::new(),
        }
    }

    pub fn binding(&self) -> &UiBinding {
        &self.binding
    }

    pub fn set_element_enabled(&mut self, entity: EntityId, enabled: bool) {
        self.element_enabled.insert(entity, enabled);
    }

    pub fn set_button_interactable(&mut self, entity: EntityId, interactable: bool) {
        self.button_interactable.insert(entity, interactable);
    }

    pub fn set_focused_button(&mut self, entity: Option<EntityId>) {
        self.focused = entity;
    }

    pub fn focusable_buttons(&self, screen_size: [f32; 2]) -> Vec<EntityId> {
        let mut focusables = Vec::new();
        if screen_size[0] <= 0.0 || screen_size[1] <= 0.0 {
            return focusables;
        }
        for element in &self.binding.elements {
            let UiElementComponent::Button(button) = element else {
                continue;
            };
            if self.is_button_focusable(button, screen_size) && !focusables.contains(&button.entity)
            {
                focusables.push(button.entity);
            }
        }
        focusables
    }

    pub fn focused_button(&self, screen_size: [f32; 2]) -> Option<EntityId> {
        let focusables = self.focusable_buttons(screen_size);
        self.focused
            .filter(|id| focusables.contains(id))
            .or_else(|| focusables.first().copied())
    }

    pub fn take_focused_clicks(&mut self) -> Vec<u64> {
        std::mem::take(&mut self.pending_clicks)
    }

    pub fn hit_test_button(&self, position: (f32, f32), screen_size: [f32; 2]) -> Option<EntityId> {
        let (px, py) = position;
        for entity in self.focusable_buttons(screen_size).into_iter().rev() {
            let Some((rect, _)) = self.element_rect(entity, screen_size) else {
                continue;
            };
            if px >= rect[0] && py >= rect[1] && px <= rect[0] + rect[2] && py <= rect[1] + rect[3]
            {
                return Some(entity);
            }
        }
        None
    }

    pub fn handle_focus(&mut self, input: &InputState, screen_size: [f32; 2]) -> Vec<EntityId> {
        let focusables = self.focusable_buttons(screen_size);
        let current = self
            .focused
            .filter(|id| focusables.contains(id))
            .or_else(|| focusables.first().copied());
        self.focused = current;

        let stick_x = input.gamepad_axis(GAMEPAD_AXIS_LEFT_X);
        let stick_y = input.gamepad_axis(GAMEPAD_AXIS_LEFT_Y);
        let horizontal = input.axis_raw("Horizontal").unwrap_or(stick_x);
        let vertical = input.axis_raw("Vertical").unwrap_or(stick_y);
        let prev_x = self.focus_axis_prev[0];
        let prev_y = self.focus_axis_prev[1];
        self.focus_axis_prev = [horizontal, vertical];

        if !focusables.is_empty() {
            let shift_down =
                input.is_key_down(KEY_LEFT_SHIFT) || input.is_key_down(KEY_RIGHT_SHIFT);
            if input.is_key_pressed(KEY_TAB) {
                let step = if shift_down { -1 } else { 1 };
                self.focused = Some(step_focus(&focusables, current, step));
            } else if let Some(direction) =
                navigation_direction(input, [prev_x, prev_y], [horizontal, vertical])
            {
                self.focused =
                    Some(self.navigate_direction(&focusables, current, direction, screen_size));
            }
        }

        let submit = input
            .button_state("Submit")
            .is_some_and(|state| state & STATE_PRESSED != 0)
            || input.is_key_pressed(KEY_RETURN)
            || input.is_key_pressed(KEY_SPACE)
            || input.is_gamepad_button_pressed(GAMEPAD_BUTTON_SOUTH);
        let mut clicked = Vec::new();
        if submit && let Some(entity) = self.focused {
            self.pending_clicks.push(entity.as_u64());
            clicked.push(entity);
        }
        clicked
    }

    pub fn draw_items(&self, screen_size: [f32; 2]) -> Vec<UiDrawItem> {
        self.draw_items_with(screen_size, &HashMap::new())
    }

    pub fn draw_items_with(
        &self,
        screen_size: [f32; 2],
        texts: &HashMap<EntityId, TextOverride>,
    ) -> Vec<UiDrawItem> {
        let mut items = Vec::new();
        if screen_size[0] <= 0.0 || screen_size[1] <= 0.0 {
            return items;
        }
        for element in &self.binding.elements {
            let (rect_px, scale) = match self.element_rect(element.entity(), screen_size) {
                Some(resolved) => resolved,
                None => continue,
            };
            if !self.is_hierarchy_visible(element.entity()) {
                continue;
            }
            match element {
                UiElementComponent::Image(image) => {
                    if !self.is_element_enabled(image.entity, image.enabled) {
                        continue;
                    }
                    items.push(UiDrawItem {
                        rect_px,
                        color: image.color,
                        sprite: image.sprite_guid.as_ref().map(|guid| SpriteRef {
                            texture_guid: guid.clone(),
                            uv_rect: [0.0, 0.0, 1.0, 1.0],
                        }),
                        text: None,
                    });
                }
                UiElementComponent::Text(text) => {
                    if !self.is_element_enabled(text.entity, text.enabled) {
                        continue;
                    }
                    let state = texts.get(&text.entity);
                    let content = state.map_or(text.text.as_str(), |state| state.text.as_str());
                    let color = state.map_or(text.color, |state| state.color);
                    let font_size = state.map_or(text.font_size, |state| state.font_size);
                    items.push(UiDrawItem {
                        rect_px,
                        color,
                        sprite: None,
                        text: Some(UiTextItem {
                            text: content.to_string(),
                            x: rect_px[0],
                            y: rect_px[1],
                            size_px: font_size * scale,
                            color,
                        }),
                    });
                }
                UiElementComponent::Button(_) => {}
            }
        }
        if let Some(focused_id) = self.focused_button(screen_size)
            && let Some((rect_px, scale)) = self.element_rect(focused_id, screen_size)
        {
            let thickness = (FOCUS_BORDER_THICKNESS * scale).round().max(1.0);
            for edge_rect in focus_border_rects(rect_px, thickness) {
                items.push(UiDrawItem {
                    rect_px: edge_rect,
                    color: FOCUS_INDICATOR_COLOR,
                    sprite: None,
                    text: None,
                });
            }
        }
        items
    }

    fn is_element_enabled(&self, entity: EntityId, default_enabled: bool) -> bool {
        self.element_enabled
            .get(&entity)
            .copied()
            .unwrap_or(default_enabled)
    }

    fn is_hierarchy_visible(&self, entity: EntityId) -> bool {
        let Some(chain) = self.chain_to_canvas(entity) else {
            return false;
        };
        for ancestor in chain {
            for element in &self.binding.elements {
                if let UiElementComponent::Image(image) = element
                    && image.entity == ancestor
                    && !self.is_element_enabled(image.entity, image.enabled)
                {
                    return false;
                }
            }
        }
        true
    }

    fn is_button_focusable(&self, button: &UiButtonComponent, screen_size: [f32; 2]) -> bool {
        let interactable = self
            .button_interactable
            .get(&button.entity)
            .copied()
            .unwrap_or(button.interactable);
        if !interactable {
            return false;
        }
        let Some((rect_px, _)) = self.element_rect(button.entity, screen_size) else {
            return false;
        };
        if rect_px[2] <= 0.0 || rect_px[3] <= 0.0 {
            return false;
        }
        if !self.is_hierarchy_visible(button.entity) {
            return false;
        }
        if let Some(target) = button.target_graphic {
            for element in &self.binding.elements {
                match element {
                    UiElementComponent::Image(image) if image.entity == target => {
                        if !self.is_element_enabled(image.entity, image.enabled) {
                            return false;
                        }
                    }
                    UiElementComponent::Text(text) if text.entity == target => {
                        if !self.is_element_enabled(text.entity, text.enabled) {
                            return false;
                        }
                    }
                    _ => {}
                }
            }
        }
        true
    }

    fn navigate_direction(
        &self,
        focusables: &[EntityId],
        current: Option<EntityId>,
        direction: [f32; 2],
        screen_size: [f32; 2],
    ) -> EntityId {
        let Some(current_id) = current else {
            return focusables[0];
        };
        let Some((current_rect, _)) = self.element_rect(current_id, screen_size) else {
            return step_focus(
                focusables,
                Some(current_id),
                if direction[0] > 0.0 || direction[1] > 0.0 {
                    1
                } else {
                    -1
                },
            );
        };
        let current_center = [
            current_rect[0] + current_rect[2] * 0.5,
            current_rect[1] + current_rect[3] * 0.5,
        ];
        let mut best: Option<(EntityId, f32)> = None;
        for &candidate in focusables {
            if candidate == current_id {
                continue;
            }
            let Some((rect, _)) = self.element_rect(candidate, screen_size) else {
                continue;
            };
            let center = [rect[0] + rect[2] * 0.5, rect[1] + rect[3] * 0.5];
            let dx = center[0] - current_center[0];
            let dy = center[1] - current_center[1];
            let dist_sq = dx * dx + dy * dy;
            if dist_sq <= f32::EPSILON {
                continue;
            }
            let dist = dist_sq.sqrt();
            let dot = (dx * direction[0] + dy * direction[1]) / dist;
            if dot <= 0.1 {
                continue;
            }
            let score = dot / dist;
            if best.is_none_or(|(_, best_score)| score > best_score) {
                best = Some((candidate, score));
            }
        }
        if let Some((winner, _)) = best {
            return winner;
        }
        let fallback_step = if direction[0] > 0.0 || direction[1] > 0.0 {
            1
        } else {
            -1
        };
        step_focus(focusables, Some(current_id), fallback_step)
    }

    /// The pixel rectangle of one element under its canvas and the scale of
    /// that canvas, in screen pixels with the origin at the top left.
    pub fn element_rect(&self, entity: EntityId, screen_size: [f32; 2]) -> Option<([f32; 4], f32)> {
        if screen_size[0] <= 0.0 || screen_size[1] <= 0.0 {
            return None;
        }
        let chain = self.chain_to_canvas(entity)?;
        let canvas = self
            .binding
            .canvases
            .iter()
            .find(|canvas| canvas.entity == chain[0])?;
        if canvas.render_mode != CanvasRenderMode::ScreenSpaceOverlay {
            return None;
        }
        let scale = canvas_scale_factor(canvas, screen_size);
        if scale <= 0.0 {
            return None;
        }
        let canvas_size = [screen_size[0] / scale, screen_size[1] / scale];
        let mut rect = CanvasRect {
            min: [0.0, 0.0],
            size: canvas_size,
        };
        for node in chain.iter().skip(1) {
            let transform = match self.rects.get(node) {
                Some(transform) => transform,
                None => continue,
            };
            rect = resolve_canvas_rect(transform, rect);
        }
        Some((canvas_rect_to_pixels(rect, canvas_size, scale), scale))
    }

    fn chain_to_canvas(&self, entity: EntityId) -> Option<Vec<EntityId>> {
        let mut chain = Vec::new();
        let mut current = Some(entity);
        while let Some(node) = current {
            if chain.len() >= MAX_HIERARCHY_DEPTH {
                return None;
            }
            chain.push(node);
            if self
                .binding
                .canvases
                .iter()
                .any(|canvas| canvas.entity == node)
            {
                chain.reverse();
                return Some(chain);
            }
            current = self.rects.get(&node).and_then(|rect| rect.parent);
        }
        None
    }
}

fn step_focus(focusables: &[EntityId], current: Option<EntityId>, step: i32) -> EntityId {
    let index = current
        .and_then(|id| focusables.iter().position(|&candidate| candidate == id))
        .unwrap_or(0);
    let next = (index as i32 + step).rem_euclid(focusables.len() as i32) as usize;
    focusables[next]
}

fn navigation_direction(
    input: &InputState,
    prev_axes: [f64; 2],
    axes: [f64; 2],
) -> Option<[f32; 2]> {
    if input.is_key_pressed(KEY_RIGHT_ARROW)
        || input.is_gamepad_button_pressed(GAMEPAD_BUTTON_DPAD_RIGHT)
        || (prev_axes[0] < STICK_NAVIGATION_THRESHOLD && axes[0] >= STICK_NAVIGATION_THRESHOLD)
    {
        return Some([1.0, 0.0]);
    }
    if input.is_key_pressed(KEY_LEFT_ARROW)
        || input.is_gamepad_button_pressed(GAMEPAD_BUTTON_DPAD_LEFT)
        || (prev_axes[0] > -STICK_NAVIGATION_THRESHOLD && axes[0] <= -STICK_NAVIGATION_THRESHOLD)
    {
        return Some([-1.0, 0.0]);
    }
    if input.is_key_pressed(KEY_DOWN_ARROW)
        || input.is_gamepad_button_pressed(GAMEPAD_BUTTON_DPAD_DOWN)
        || (prev_axes[1] > -STICK_NAVIGATION_THRESHOLD && axes[1] <= -STICK_NAVIGATION_THRESHOLD)
    {
        return Some([0.0, 1.0]);
    }
    if input.is_key_pressed(KEY_UP_ARROW)
        || input.is_gamepad_button_pressed(GAMEPAD_BUTTON_DPAD_UP)
        || (prev_axes[1] < STICK_NAVIGATION_THRESHOLD && axes[1] >= STICK_NAVIGATION_THRESHOLD)
    {
        return Some([0.0, -1.0]);
    }
    None
}

fn canvas_rect_to_pixels(rect: CanvasRect, canvas_size: [f32; 2], scale: f32) -> [f32; 4] {
    let x = rect.min[0] * scale;
    let width = rect.size[0] * scale;
    let top = (canvas_size[1] - rect.min[1] - rect.size[1]) * scale;
    let height = rect.size[1] * scale;
    [x, top, width, height]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{
        GAMEPAD_AXIS_LEFT_X, GAMEPAD_AXIS_LEFT_Y, GAMEPAD_BUTTON_DPAD_DOWN,
        GAMEPAD_BUTTON_DPAD_RIGHT, GAMEPAD_BUTTON_DPAD_UP, GAMEPAD_BUTTON_SOUTH, InputState,
        KEY_DOWN_ARROW, KEY_LEFT_ARROW, KEY_LEFT_SHIFT, KEY_RETURN, KEY_RIGHT_ARROW, KEY_SPACE,
        KEY_TAB, KEY_UP_ARROW,
    };
    use unitas_asset::scene::{
        CanvasScalerComponent, UiButtonComponent, UiImageComponent, UiTextComponent,
    };
    use unitas_render::image::Rgba8;
    use unitas_render::ui_pass::{FOCUS_INDICATOR_COLOR, UiTarget, composite_ui};

    const SCREEN: [f32; 2] = [800.0, 600.0];
    const CANVAS_ID: EntityId = EntityId::new(100);
    const TITLE_PANEL_ID: EntityId = EntityId::new(200);
    const PLAY_BUTTON_ID: EntityId = EntityId::new(210);
    const QUIT_BUTTON_ID: EntityId = EntityId::new(220);
    const DISABLED_BUTTON_ID: EntityId = EntityId::new(230);
    const RESULTS_PANEL_ID: EntityId = EntityId::new(300);
    const RETRY_BUTTON_ID: EntityId = EntityId::new(310);
    const ARENA_SELECT_BUTTON_ID: EntityId = EntityId::new(320);

    fn centered_rect(
        entity: EntityId,
        parent: Option<EntityId>,
        pos: [f32; 2],
        size: [f32; 2],
    ) -> RectTransformComponent {
        RectTransformComponent {
            entity,
            parent,
            anchor_min: [0.5, 0.5],
            anchor_max: [0.5, 0.5],
            anchored_position: pos,
            size_delta: size,
            pivot: [0.5, 0.5],
        }
    }

    fn sample_canvas() -> CanvasState {
        let binding = UiBinding {
            canvases: vec![CanvasComponent {
                entity: CANVAS_ID,
                render_mode: CanvasRenderMode::ScreenSpaceOverlay,
                sorting_order: 0,
                scaler: Some(CanvasScalerComponent {
                    ui_scale_mode: CanvasScaleMode::ScaleWithScreenSize,
                    reference_resolution: SCREEN,
                    screen_match_mode: CanvasMatchMode::MatchWidthOrHeight,
                    match_width_or_height: 0.5,
                }),
            }],
            rect_transforms: vec![
                centered_rect(CANVAS_ID, None, [0.0, 0.0], SCREEN),
                centered_rect(TITLE_PANEL_ID, Some(CANVAS_ID), [0.0, 0.0], [420.0, 260.0]),
                centered_rect(
                    PLAY_BUTTON_ID,
                    Some(TITLE_PANEL_ID),
                    [0.0, 10.0],
                    [200.0, 48.0],
                ),
                centered_rect(
                    QUIT_BUTTON_ID,
                    Some(TITLE_PANEL_ID),
                    [0.0, -50.0],
                    [200.0, 48.0],
                ),
                centered_rect(
                    DISABLED_BUTTON_ID,
                    Some(TITLE_PANEL_ID),
                    [0.0, -110.0],
                    [200.0, 48.0],
                ),
                centered_rect(
                    RESULTS_PANEL_ID,
                    Some(CANVAS_ID),
                    [0.0, 0.0],
                    [440.0, 320.0],
                ),
                centered_rect(
                    RETRY_BUTTON_ID,
                    Some(RESULTS_PANEL_ID),
                    [-100.0, -70.0],
                    [180.0, 44.0],
                ),
                centered_rect(
                    ARENA_SELECT_BUTTON_ID,
                    Some(RESULTS_PANEL_ID),
                    [100.0, -70.0],
                    [180.0, 44.0],
                ),
            ],
            elements: vec![
                UiElementComponent::Image(UiImageComponent {
                    entity: TITLE_PANEL_ID,
                    enabled: true,
                    color: [0.08, 0.1, 0.16, 0.88],
                    sprite_guid: None,
                }),
                UiElementComponent::Image(UiImageComponent {
                    entity: PLAY_BUTTON_ID,
                    enabled: true,
                    color: [0.2, 0.65, 0.32, 1.0],
                    sprite_guid: None,
                }),
                UiElementComponent::Button(UiButtonComponent {
                    entity: PLAY_BUTTON_ID,
                    target_graphic: Some(PLAY_BUTTON_ID),
                    interactable: true,
                }),
                UiElementComponent::Text(UiTextComponent {
                    entity: PLAY_BUTTON_ID,
                    enabled: true,
                    text: "PLAY".to_string(),
                    font_size: 22.0,
                    color: [1.0, 1.0, 1.0, 1.0],
                }),
                UiElementComponent::Image(UiImageComponent {
                    entity: QUIT_BUTTON_ID,
                    enabled: true,
                    color: [0.55, 0.2, 0.22, 1.0],
                    sprite_guid: None,
                }),
                UiElementComponent::Button(UiButtonComponent {
                    entity: QUIT_BUTTON_ID,
                    target_graphic: Some(QUIT_BUTTON_ID),
                    interactable: true,
                }),
                UiElementComponent::Image(UiImageComponent {
                    entity: DISABLED_BUTTON_ID,
                    enabled: true,
                    color: [0.3, 0.3, 0.3, 1.0],
                    sprite_guid: None,
                }),
                UiElementComponent::Button(UiButtonComponent {
                    entity: DISABLED_BUTTON_ID,
                    target_graphic: Some(DISABLED_BUTTON_ID),
                    interactable: false,
                }),
                UiElementComponent::Image(UiImageComponent {
                    entity: RESULTS_PANEL_ID,
                    enabled: false,
                    color: [0.08, 0.1, 0.16, 0.92],
                    sprite_guid: None,
                }),
                UiElementComponent::Image(UiImageComponent {
                    entity: RETRY_BUTTON_ID,
                    enabled: true,
                    color: [0.2, 0.65, 0.32, 1.0],
                    sprite_guid: None,
                }),
                UiElementComponent::Button(UiButtonComponent {
                    entity: RETRY_BUTTON_ID,
                    target_graphic: Some(RETRY_BUTTON_ID),
                    interactable: true,
                }),
                UiElementComponent::Image(UiImageComponent {
                    entity: ARENA_SELECT_BUTTON_ID,
                    enabled: true,
                    color: [0.22, 0.45, 0.72, 1.0],
                    sprite_guid: None,
                }),
                UiElementComponent::Button(UiButtonComponent {
                    entity: ARENA_SELECT_BUTTON_ID,
                    target_graphic: Some(ARENA_SELECT_BUTTON_ID),
                    interactable: true,
                }),
            ],
        };
        CanvasState::new(binding)
    }

    #[test]
    fn focusable_registry_filters_disabled_and_hidden_controls_and_auto_focuses_first() {
        let mut canvas = sample_canvas();
        assert_eq!(
            canvas.focusable_buttons(SCREEN),
            vec![PLAY_BUTTON_ID, QUIT_BUTTON_ID]
        );
        assert_eq!(canvas.focused_button(SCREEN), Some(PLAY_BUTTON_ID));

        canvas.set_element_enabled(TITLE_PANEL_ID, false);
        canvas.set_element_enabled(RESULTS_PANEL_ID, true);
        assert_eq!(
            canvas.focusable_buttons(SCREEN),
            vec![RETRY_BUTTON_ID, ARENA_SELECT_BUTTON_ID]
        );
        assert_eq!(canvas.focused_button(SCREEN), Some(RETRY_BUTTON_ID));
    }

    #[test]
    fn directional_and_tab_navigation_move_focus_deterministically_across_panels() {
        let mut canvas = sample_canvas();
        let mut input = InputState::with_default_axes();
        input.connect_gamepad(1);

        input.queue_key_event(KEY_DOWN_ARROW, true);
        input.begin_frame(1.0 / 60.0);
        assert!(canvas.handle_focus(&input, SCREEN).is_empty());
        assert_eq!(canvas.focused_button(SCREEN), Some(QUIT_BUTTON_ID));
        input.queue_key_event(KEY_DOWN_ARROW, false);
        input.begin_frame(1.0 / 60.0);

        input.queue_key_event(KEY_UP_ARROW, true);
        input.begin_frame(1.0 / 60.0);
        canvas.handle_focus(&input, SCREEN);
        assert_eq!(canvas.focused_button(SCREEN), Some(PLAY_BUTTON_ID));
        input.queue_key_event(KEY_UP_ARROW, false);
        input.begin_frame(1.0 / 60.0);

        input.set_gamepad_button(GAMEPAD_BUTTON_DPAD_DOWN, true);
        input.begin_frame(1.0 / 60.0);
        canvas.handle_focus(&input, SCREEN);
        assert_eq!(canvas.focused_button(SCREEN), Some(QUIT_BUTTON_ID));
        input.set_gamepad_button(GAMEPAD_BUTTON_DPAD_DOWN, false);
        input.begin_frame(1.0 / 60.0);
        canvas.handle_focus(&input, SCREEN);

        input.set_gamepad_axis(GAMEPAD_AXIS_LEFT_Y, 0.9);
        input.begin_frame(1.0 / 60.0);
        canvas.handle_focus(&input, SCREEN);
        assert_eq!(canvas.focused_button(SCREEN), Some(PLAY_BUTTON_ID));
        input.set_gamepad_axis(GAMEPAD_AXIS_LEFT_Y, 0.0);
        input.begin_frame(1.0 / 60.0);
        canvas.handle_focus(&input, SCREEN);

        input.queue_key_event(KEY_TAB, true);
        input.begin_frame(1.0 / 60.0);
        canvas.handle_focus(&input, SCREEN);
        assert_eq!(canvas.focused_button(SCREEN), Some(QUIT_BUTTON_ID));
        input.queue_key_event(KEY_TAB, false);
        input.begin_frame(1.0 / 60.0);

        input.queue_key_event(KEY_LEFT_SHIFT, true);
        input.queue_key_event(KEY_TAB, true);
        input.begin_frame(1.0 / 60.0);
        canvas.handle_focus(&input, SCREEN);
        assert_eq!(canvas.focused_button(SCREEN), Some(PLAY_BUTTON_ID));
        input.queue_key_event(KEY_TAB, false);
        input.queue_key_event(KEY_LEFT_SHIFT, false);
        input.begin_frame(1.0 / 60.0);

        canvas.set_element_enabled(TITLE_PANEL_ID, false);
        canvas.set_element_enabled(RESULTS_PANEL_ID, true);
        assert_eq!(canvas.focused_button(SCREEN), Some(RETRY_BUTTON_ID));

        input.queue_key_event(KEY_RIGHT_ARROW, true);
        input.begin_frame(1.0 / 60.0);
        canvas.handle_focus(&input, SCREEN);
        assert_eq!(canvas.focused_button(SCREEN), Some(ARENA_SELECT_BUTTON_ID));
        input.queue_key_event(KEY_RIGHT_ARROW, false);
        input.begin_frame(1.0 / 60.0);

        input.queue_key_event(KEY_LEFT_ARROW, true);
        input.begin_frame(1.0 / 60.0);
        canvas.handle_focus(&input, SCREEN);
        assert_eq!(canvas.focused_button(SCREEN), Some(RETRY_BUTTON_ID));
        input.queue_key_event(KEY_LEFT_ARROW, false);
        input.begin_frame(1.0 / 60.0);

        input.set_gamepad_button(GAMEPAD_BUTTON_DPAD_RIGHT, true);
        input.begin_frame(1.0 / 60.0);
        canvas.handle_focus(&input, SCREEN);
        assert_eq!(canvas.focused_button(SCREEN), Some(ARENA_SELECT_BUTTON_ID));
        input.set_gamepad_button(GAMEPAD_BUTTON_DPAD_RIGHT, false);
        input.begin_frame(1.0 / 60.0);
        canvas.handle_focus(&input, SCREEN);

        input.set_gamepad_axis(GAMEPAD_AXIS_LEFT_X, -0.85);
        input.begin_frame(1.0 / 60.0);
        canvas.handle_focus(&input, SCREEN);
        assert_eq!(canvas.focused_button(SCREEN), Some(RETRY_BUTTON_ID));
        input.set_gamepad_axis(GAMEPAD_AXIS_LEFT_X, 0.0);
        input.begin_frame(1.0 / 60.0);
        canvas.handle_focus(&input, SCREEN);

        input.set_gamepad_button(GAMEPAD_BUTTON_DPAD_UP, true);
        input.begin_frame(1.0 / 60.0);
        canvas.handle_focus(&input, SCREEN);
        assert_eq!(canvas.focused_button(SCREEN), Some(ARENA_SELECT_BUTTON_ID));
    }

    #[test]
    fn draw_items_include_visible_focus_indicator_border_around_focused_button() {
        let mut canvas = sample_canvas();
        let items = canvas.draw_items(SCREEN);
        let focus_items: Vec<&UiDrawItem> = items
            .iter()
            .filter(|item| item.color == FOCUS_INDICATOR_COLOR)
            .collect();
        assert_eq!(focus_items.len(), 4);
        let (play_rect, _) = canvas
            .element_rect(PLAY_BUTTON_ID, SCREEN)
            .unwrap_or(([0.0; 4], 1.0));
        assert_eq!(focus_items[0].rect_px[0], play_rect[0]);
        assert_eq!(focus_items[0].rect_px[1], play_rect[1]);
        assert_eq!(focus_items[0].rect_px[2], play_rect[2]);

        let mut image = Rgba8::new(800, 600);
        let mut target = UiTarget {
            image: &mut image,
            ids: None,
        };
        composite_ui(&mut target, &items, 1, None);
        let border_x = play_rect[0] as usize + 1;
        let border_y = play_rect[1] as usize;
        let pixel_idx = (border_y * 800 + border_x) * 4;
        assert_eq!(image.pixels[pixel_idx], 255);
        assert_eq!(image.pixels[pixel_idx + 1], 230);
        assert_eq!(image.pixels[pixel_idx + 2], 0);

        canvas.set_focused_button(Some(QUIT_BUTTON_ID));
        let moved_items = canvas.draw_items(SCREEN);
        let moved_focus: Vec<&UiDrawItem> = moved_items
            .iter()
            .filter(|item| item.color == FOCUS_INDICATOR_COLOR)
            .collect();
        assert_eq!(moved_focus.len(), 4);
        let (quit_rect, _) = canvas
            .element_rect(QUIT_BUTTON_ID, SCREEN)
            .unwrap_or(([0.0; 4], 1.0));
        assert_eq!(moved_focus[0].rect_px[1], quit_rect[1]);
    }

    #[test]
    fn submit_keys_and_gamepad_south_enqueue_click_identical_to_pointer_hit_test() {
        let mut canvas = sample_canvas();
        let mut input = InputState::with_default_axes();
        input.connect_gamepad(1);

        let (play_rect, _) = canvas
            .element_rect(PLAY_BUTTON_ID, SCREEN)
            .unwrap_or(([0.0; 4], 1.0));
        let pointer_center = (
            play_rect[0] + play_rect[2] * 0.5,
            play_rect[1] + play_rect[3] * 0.5,
        );
        let pointer_hit = canvas.hit_test_button(pointer_center, SCREEN);
        assert_eq!(pointer_hit, Some(PLAY_BUTTON_ID));

        input.queue_key_event(KEY_RETURN, true);
        input.begin_frame(1.0 / 60.0);
        let clicks = canvas.handle_focus(&input, SCREEN);
        assert_eq!(clicks, vec![PLAY_BUTTON_ID]);
        assert_eq!(canvas.take_focused_clicks(), vec![PLAY_BUTTON_ID.as_u64()]);
        input.queue_key_event(KEY_RETURN, false);
        input.begin_frame(1.0 / 60.0);

        input.queue_key_event(KEY_DOWN_ARROW, true);
        input.begin_frame(1.0 / 60.0);
        assert!(canvas.handle_focus(&input, SCREEN).is_empty());
        assert_eq!(canvas.focused_button(SCREEN), Some(QUIT_BUTTON_ID));
        input.queue_key_event(KEY_DOWN_ARROW, false);
        input.begin_frame(1.0 / 60.0);

        input.queue_key_event(KEY_SPACE, true);
        input.begin_frame(1.0 / 60.0);
        assert_eq!(canvas.handle_focus(&input, SCREEN), vec![QUIT_BUTTON_ID]);
        assert_eq!(canvas.take_focused_clicks(), vec![QUIT_BUTTON_ID.as_u64()]);
        input.queue_key_event(KEY_SPACE, false);
        input.begin_frame(1.0 / 60.0);

        input.set_gamepad_button(GAMEPAD_BUTTON_SOUTH, true);
        input.begin_frame(1.0 / 60.0);
        assert_eq!(canvas.handle_focus(&input, SCREEN), vec![QUIT_BUTTON_ID]);
        assert_eq!(canvas.take_focused_clicks(), vec![QUIT_BUTTON_ID.as_u64()]);
    }
}
