use std::collections::HashMap;
use std::sync::{Mutex, OnceLock, RwLock};

use unitas_asset::scene::{
    CanvasComponent, CanvasMatchMode, CanvasRenderMode, CanvasScaleMode, ContentSizeFitMode,
    LayoutGroupAxis, RectTransformComponent, SpriteDrawMode, UiBinding, UiButtonComponent,
    UiContentSizeFitterComponent, UiElementComponent, UiImageComponent, UiLayoutGroupComponent,
    UiTextComponent,
};
use unitas_core::EntityId;
use unitas_render::glyphs::{FontLibrary, TextStyle, layout_text};
use unitas_render::ui_pass::{
    FOCUS_INDICATOR_COLOR, HorizontalAlign, VerticalAlign, focus_border_rects, ui_sprite_size,
};

use crate::input::{
    GAMEPAD_AXIS_LEFT_X, GAMEPAD_AXIS_LEFT_Y, GAMEPAD_BUTTON_DPAD_DOWN, GAMEPAD_BUTTON_DPAD_LEFT,
    GAMEPAD_BUTTON_DPAD_RIGHT, GAMEPAD_BUTTON_DPAD_UP, GAMEPAD_BUTTON_SOUTH, InputState,
    KEY_BACKSPACE, KEY_DELETE, KEY_DOWN_ARROW, KEY_ESCAPE, KEY_LEFT_ARROW, KEY_LEFT_SHIFT,
    KEY_RETURN, KEY_RIGHT_ARROW, KEY_RIGHT_SHIFT, KEY_SPACE, KEY_TAB, KEY_UP_ARROW, STATE_PRESSED,
};
use crate::ui::UiTextItem;

pub use unitas_render::ui_pass::SpriteRef;

pub const KEY_HOME: u32 = 278;
pub const KEY_END: u32 = 279;

#[derive(Clone, Debug, PartialEq)]
pub struct UiInputFieldComponent {
    pub entity: EntityId,
    pub text: String,
    pub placeholder: String,
    pub caret_position: usize,
    pub interactable: bool,
    pub character_limit: usize,
    pub color: [f32; 4],
    pub placeholder_color: [f32; 4],
    pub background_color: [f32; 4],
    pub font_size: f32,
    pub enabled: bool,
    pub editing_original_text: Option<String>,
    pub was_canceled: bool,
}

impl Default for UiInputFieldComponent {
    fn default() -> Self {
        Self {
            entity: EntityId::new(0),
            text: String::new(),
            placeholder: String::new(),
            caret_position: 0,
            interactable: true,
            character_limit: 0,
            color: [1.0, 1.0, 1.0, 1.0],
            placeholder_color: [0.6, 0.6, 0.6, 0.85],
            background_color: [0.16, 0.18, 0.24, 1.0],
            font_size: 16.0,
            enabled: true,
            editing_original_text: None,
            was_canceled: false,
        }
    }
}

impl UiInputFieldComponent {
    pub fn clamp_caret(text: &str, pos: usize) -> usize {
        let clamped = pos.min(text.len());
        if text.is_char_boundary(clamped) {
            return clamped;
        }
        let mut idx = clamped;
        while idx > 0 && !text.is_char_boundary(idx) {
            idx -= 1;
        }
        idx
    }

    pub fn char_count(&self) -> usize {
        self.text.chars().count()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputFieldValueChangedEvent {
    pub entity: EntityId,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputFieldEndEditEvent {
    pub entity: EntityId,
    pub text: String,
    pub canceled: bool,
}

const MAX_HIERARCHY_DEPTH: usize = 64;
const FOCUS_BORDER_THICKNESS: f32 = 2.0;
const STICK_NAVIGATION_THRESHOLD: f64 = 0.5;
const FULL_UV: [f32; 4] = [0.0, 0.0, 1.0, 1.0];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TextAnchor {
    #[default]
    UpperLeft = 0,
    UpperCenter = 1,
    UpperRight = 2,
    MiddleLeft = 3,
    MiddleCenter = 4,
    MiddleRight = 5,
    LowerLeft = 6,
    LowerCenter = 7,
    LowerRight = 8,
}

impl TextAnchor {
    pub const fn from_u32(code: u32) -> Option<Self> {
        match code {
            0 => Some(Self::UpperLeft),
            1 => Some(Self::UpperCenter),
            2 => Some(Self::UpperRight),
            3 => Some(Self::MiddleLeft),
            4 => Some(Self::MiddleCenter),
            5 => Some(Self::MiddleRight),
            6 => Some(Self::LowerLeft),
            7 => Some(Self::LowerCenter),
            8 => Some(Self::LowerRight),
            _ => None,
        }
    }

    pub const fn horizontal(self) -> HorizontalAlign {
        match self {
            Self::UpperLeft | Self::MiddleLeft | Self::LowerLeft => HorizontalAlign::Left,
            Self::UpperCenter | Self::MiddleCenter | Self::LowerCenter => HorizontalAlign::Center,
            Self::UpperRight | Self::MiddleRight | Self::LowerRight => HorizontalAlign::Right,
        }
    }

    pub const fn vertical(self) -> VerticalAlign {
        match self {
            Self::UpperLeft | Self::UpperCenter | Self::UpperRight => VerticalAlign::Top,
            Self::MiddleLeft | Self::MiddleCenter | Self::MiddleRight => VerticalAlign::Middle,
            Self::LowerLeft | Self::LowerCenter | Self::LowerRight => VerticalAlign::Bottom,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum HorizontalWrapMode {
    Wrap = 0,
    #[default]
    Overflow = 1,
}

impl HorizontalWrapMode {
    pub const fn from_u32(code: u32) -> Option<Self> {
        match code {
            0 => Some(Self::Wrap),
            1 => Some(Self::Overflow),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum VerticalWrapMode {
    Truncate = 0,
    #[default]
    Overflow = 1,
}

impl VerticalWrapMode {
    pub const fn from_u32(code: u32) -> Option<Self> {
        match code {
            0 => Some(Self::Truncate),
            1 => Some(Self::Overflow),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextLayoutConfig {
    pub alignment: TextAnchor,
    pub horizontal_overflow: HorizontalWrapMode,
    pub vertical_overflow: VerticalWrapMode,
    pub line_spacing: f32,
}

impl Default for TextLayoutConfig {
    fn default() -> Self {
        Self {
            alignment: TextAnchor::UpperLeft,
            horizontal_overflow: HorizontalWrapMode::Overflow,
            vertical_overflow: VerticalWrapMode::Overflow,
            line_spacing: 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UiDrawItem {
    pub rect_px: [f32; 4],
    pub color: [f32; 4],
    pub sprite: Option<SpriteRef>,
    pub text: Option<UiTextItem>,
    pub text_layout: TextLayoutConfig,
    pub text_clip_rect: Option<[f32; 4]>,
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
        self.text.as_ref().map(|item| {
            let wrap = self.text_layout.horizontal_overflow == HorizontalWrapMode::Wrap;
            unitas_render::ui_pass::UiText {
                text: &item.text,
                x: item.x,
                y: item.y,
                size_px: item.size_px,
                color: item.color,
                horizontal_align: self.text_layout.alignment.horizontal(),
                vertical_align: self.text_layout.alignment.vertical(),
                wrap,
                max_width: wrap.then_some(self.rect_px[2]),
                line_spacing: self.text_layout.line_spacing,
                clip_rect: self.text_clip_rect,
            }
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasRect {
    pub min: [f32; 2],
    pub size: [f32; 2],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpriteQuad {
    pub rect: [f32; 4],
    pub uv: [f32; 4],
}

fn clamp_sliced_borders(border: [f32; 4], rect: [f32; 4], scale: f32) -> (f32, f32, f32, f32) {
    let (mut left, mut bottom, mut right, mut top) = (
        border[0] * scale,
        border[1] * scale,
        border[2] * scale,
        border[3] * scale,
    );
    let (width, height) = (rect[2], rect[3]);
    if left + right > width {
        let difference = left - right;
        left = 0.5 * width + 0.5 * difference;
        right = 0.5 * width - 0.5 * difference;
    }
    if bottom + top > height {
        let difference = top - bottom;
        bottom = 0.5 * height - 0.5 * difference;
        top = 0.5 * height + 0.5 * difference;
    }
    (left, bottom, right, top)
}

pub fn slice_sprite_quads(
    border: [f32; 4],
    sprite_size: [f32; 2],
    rect: [f32; 4],
    uv_rect: [f32; 4],
    scale: f32,
) -> Vec<SpriteQuad> {
    let (left, bottom, right, top) = clamp_sliced_borders(border, rect, scale);
    let (width, height) = (rect[2], rect[3]);
    let (u0, v0, u1, v1) = (uv_rect[0], uv_rect[1], uv_rect[2], uv_rect[3]);
    let (u_span, v_span) = (u1 - u0, v1 - v0);
    let u_left = u0 + border[0] / sprite_size[0] * u_span;
    let u_right = u1 - border[2] / sprite_size[0] * u_span;
    let v_top = v0 + border[3] / sprite_size[1] * v_span;
    let v_bottom = v1 - border[1] / sprite_size[1] * v_span;
    let (x, y) = (rect[0], rect[1]);
    let (mid_x, end_x) = (x + left, x + width - right);
    let (mid_y, end_y) = (y + top, y + height - bottom);
    let (mid_w, mid_h) = (end_x - mid_x, end_y - mid_y);
    let quad = |r, uv| SpriteQuad { rect: r, uv };
    vec![
        quad([x, y, left, top], [u0, v0, u_left, v_top]),
        quad([mid_x, y, mid_w, top], [u_left, v0, u_right, v_top]),
        quad([end_x, y, right, top], [u_right, v0, u1, v_top]),
        quad([x, mid_y, left, mid_h], [u0, v_top, u_left, v_bottom]),
        quad(
            [mid_x, mid_y, mid_w, mid_h],
            [u_left, v_top, u_right, v_bottom],
        ),
        quad([end_x, mid_y, right, mid_h], [u_right, v_top, u1, v_bottom]),
        quad([x, end_y, left, bottom], [u0, v_bottom, u_left, v1]),
        quad(
            [mid_x, end_y, mid_w, bottom],
            [u_left, v_bottom, u_right, v1],
        ),
        quad([end_x, end_y, right, bottom], [u_right, v_bottom, u1, v1]),
    ]
}

pub fn tile_sprite_quads(
    sprite_size: [f32; 2],
    rect: [f32; 4],
    uv_rect: [f32; 4],
    scale: f32,
) -> Vec<SpriteQuad> {
    let tile_width = sprite_size[0] * scale;
    let tile_height = sprite_size[1] * scale;
    if tile_width <= 0.0 || tile_height <= 0.0 {
        return Vec::new();
    }
    let columns = (rect[2] / tile_width).ceil().max(1.0) as usize;
    let rows = (rect[3] / tile_height).ceil().max(1.0) as usize;
    let mut quads = Vec::with_capacity(columns * rows);
    for row in 0..rows {
        for column in 0..columns {
            let x = rect[0] + column as f32 * tile_width;
            let y = rect[1] + row as f32 * tile_height;
            let width = tile_width.min(rect[0] + rect[2] - x);
            let height = tile_height.min(rect[1] + rect[3] - y);
            if width <= 0.0 || height <= 0.0 {
                continue;
            }
            quads.push(SpriteQuad {
                rect: [x, y, width, height],
                uv: uv_rect,
            });
        }
    }
    quads
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextOverride {
    pub text: String,
    pub font_size: f32,
    pub color: [f32; 4],
    pub layout: TextLayoutConfig,
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
    input_fields: HashMap<EntityId, UiInputFieldComponent>,
    input_field_order: Vec<EntityId>,
    pending_value_changed: Vec<InputFieldValueChangedEvent>,
    pending_end_edit: Vec<InputFieldEndEditEvent>,
    layout_group_overrides: RwLock<HashMap<EntityId, UiLayoutGroupComponent>>,
    fitter_overrides: RwLock<HashMap<EntityId, UiContentSizeFitterComponent>>,
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
            input_fields: HashMap::new(),
            input_field_order: Vec::new(),
            pending_value_changed: Vec::new(),
            pending_end_edit: Vec::new(),
            layout_group_overrides: RwLock::new(HashMap::new()),
            fitter_overrides: RwLock::new(HashMap::new()),
        }
    }

    pub fn binding(&self) -> &UiBinding {
        &self.binding
    }

    pub fn ensure_canvas_and_rect(&mut self, entity: EntityId, screen_size: [f32; 2]) {
        if self.binding.canvases.is_empty() {
            let canvas_entity = EntityId::new(u64::MAX - 1);
            self.binding.canvases.push(CanvasComponent {
                entity: canvas_entity,
                render_mode: CanvasRenderMode::ScreenSpaceOverlay,
                sorting_order: 0,
                scaler: None,
            });
            let canvas_rect = RectTransformComponent {
                entity: canvas_entity,
                parent: None,
                anchor_min: [0.5, 0.5],
                anchor_max: [0.5, 0.5],
                anchored_position: [0.0, 0.0],
                size_delta: screen_size,
                pivot: [0.5, 0.5],
            };
            self.binding.rect_transforms.push(canvas_rect.clone());
            self.rects.insert(canvas_entity, canvas_rect);
        }
        if !self.rects.contains_key(&entity) {
            let parent = self.binding.canvases.first().map(|c| c.entity);
            let rect = RectTransformComponent {
                entity,
                parent,
                anchor_min: [0.5, 0.5],
                anchor_max: [0.5, 0.5],
                anchored_position: [0.0, 0.0],
                size_delta: [240.0, 36.0],
                pivot: [0.5, 0.5],
            };
            self.binding.rect_transforms.push(rect.clone());
            self.rects.insert(entity, rect);
        }
    }

    pub fn set_input_field(&mut self, mut field: UiInputFieldComponent) {
        field.caret_position =
            UiInputFieldComponent::clamp_caret(&field.text, field.caret_position);
        let entity = field.entity;
        if let Some(existing) = self.input_fields.get(&entity)
            && field.editing_original_text.is_none()
        {
            field.editing_original_text = existing.editing_original_text.clone();
        }
        if !self.input_field_order.contains(&entity) {
            self.input_field_order.push(entity);
        }
        self.input_fields.insert(entity, field);
    }

    pub fn input_field(&self, entity: EntityId) -> Option<&UiInputFieldComponent> {
        self.input_fields.get(&entity)
    }

    pub fn input_field_mut(&mut self, entity: EntityId) -> Option<&mut UiInputFieldComponent> {
        self.input_fields.get_mut(&entity)
    }

    pub fn set_input_field_text(&mut self, entity: EntityId, text: &str, notify: bool) -> bool {
        let Some(field) = self.input_fields.get_mut(&entity) else {
            return false;
        };
        let mut truncated = text.to_string();
        if field.character_limit > 0 && truncated.chars().count() > field.character_limit {
            truncated = truncated.chars().take(field.character_limit).collect();
        }
        let changed = field.text != truncated;
        field.text = truncated;
        field.caret_position = field.text.len();
        if changed && notify {
            self.pending_value_changed
                .push(InputFieldValueChangedEvent {
                    entity,
                    text: field.text.clone(),
                });
        }
        true
    }

    pub fn set_input_field_placeholder(&mut self, entity: EntityId, placeholder: &str) -> bool {
        let Some(field) = self.input_fields.get_mut(&entity) else {
            return false;
        };
        field.placeholder = placeholder.to_string();
        true
    }

    pub fn set_input_field_interactable(&mut self, entity: EntityId, interactable: bool) -> bool {
        let Some(field) = self.input_fields.get_mut(&entity) else {
            return false;
        };
        field.interactable = interactable;
        if !interactable && self.focused == Some(entity) {
            field.editing_original_text = None;
            self.focused = None;
        }
        true
    }

    pub fn set_input_field_character_limit(&mut self, entity: EntityId, limit: usize) -> bool {
        let Some(field) = self.input_fields.get_mut(&entity) else {
            return false;
        };
        field.character_limit = limit;
        if limit > 0 && field.text.chars().count() > limit {
            field.text = field.text.chars().take(limit).collect();
            field.caret_position =
                UiInputFieldComponent::clamp_caret(&field.text, field.caret_position);
        }
        true
    }

    pub fn set_input_field_caret_position(&mut self, entity: EntityId, caret: usize) -> bool {
        let Some(field) = self.input_fields.get_mut(&entity) else {
            return false;
        };
        field.caret_position = UiInputFieldComponent::clamp_caret(&field.text, caret);
        true
    }

    pub fn focus_input_field(&mut self, entity: EntityId, focused: bool) -> bool {
        let Some(field) = self.input_fields.get_mut(&entity) else {
            return false;
        };
        if focused {
            if !field.interactable || !field.enabled {
                return false;
            }
            if field.editing_original_text.is_none() {
                field.editing_original_text = Some(field.text.clone());
            }
            field.was_canceled = false;
            field.caret_position =
                UiInputFieldComponent::clamp_caret(&field.text, field.text.len());
            self.focused = Some(entity);
            true
        } else {
            let was_editing = field.editing_original_text.take().is_some();
            if self.focused == Some(entity) {
                self.focused = None;
            }
            if was_editing {
                let text = field.text.clone();
                let canceled = field.was_canceled;
                self.pending_end_edit.push(InputFieldEndEditEvent {
                    entity,
                    text,
                    canceled,
                });
            }
            true
        }
    }

    pub fn take_input_field_value_changed(&mut self) -> Vec<InputFieldValueChangedEvent> {
        std::mem::take(&mut self.pending_value_changed)
    }

    pub fn take_input_field_end_edit(&mut self) -> Vec<InputFieldEndEditEvent> {
        std::mem::take(&mut self.pending_end_edit)
    }

    pub fn set_element_enabled(&mut self, entity: EntityId, enabled: bool) {
        self.element_enabled.insert(entity, enabled);
    }

    pub fn set_button_interactable(&mut self, entity: EntityId, interactable: bool) {
        self.button_interactable.insert(entity, interactable);
    }

    pub fn set_focused_button(&mut self, entity: Option<EntityId>) {
        if let Some(prev) = self.focused
            && Some(prev) != entity
            && let Some(prev_field) = self.input_fields.get_mut(&prev)
        {
            prev_field.editing_original_text = None;
        }
        if let Some(target) = entity
            && let Some(field) = self.input_fields.get_mut(&target)
            && field.editing_original_text.is_none()
        {
            field.editing_original_text = Some(field.text.clone());
            field.was_canceled = false;
        }
        self.focused = entity;
    }

    pub fn set_layout_group(&self, group: UiLayoutGroupComponent) -> bool {
        let Ok(mut guard) = self.layout_group_overrides.write() else {
            return false;
        };
        guard.insert(group.entity, group);
        true
    }

    pub fn set_content_size_fitter(&self, fitter: UiContentSizeFitterComponent) -> bool {
        let Ok(mut guard) = self.fitter_overrides.write() else {
            return false;
        };
        guard.insert(fitter.entity, fitter);
        true
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
        for &entity in &self.input_field_order {
            if let Some(field) = self.input_fields.get(&entity)
                && self.is_input_field_focusable(field, screen_size)
                && !focusables.contains(&entity)
            {
                focusables.push(entity);
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
            if self.input_fields.contains_key(&entity) {
                continue;
            }
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

    pub fn hit_test_input_field(
        &self,
        position: (f32, f32),
        screen_size: [f32; 2],
    ) -> Option<EntityId> {
        let (px, py) = position;
        for &entity in self.input_field_order.iter().rev() {
            let Some(field) = self.input_fields.get(&entity) else {
                continue;
            };
            if !self.is_input_field_focusable(field, screen_size) {
                continue;
            }
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

    fn current_focus_target(&mut self, focusables: &[EntityId]) -> Option<EntityId> {
        let current = self
            .focused
            .filter(|id| focusables.contains(id))
            .or_else(|| focusables.first().copied());
        if let Some(target) = current
            && let Some(field) = self.input_fields.get_mut(&target)
            && field.editing_original_text.is_none()
        {
            field.editing_original_text = Some(field.text.clone());
            field.was_canceled = false;
        }
        current
    }

    fn collect_submit_click(&mut self, input: &InputState) -> Vec<EntityId> {
        let submit = input
            .button_state("Submit")
            .is_some_and(|state| state & STATE_PRESSED != 0)
            || input.is_key_pressed(KEY_RETURN)
            || input.is_key_pressed(KEY_SPACE)
            || input.is_gamepad_button_pressed(GAMEPAD_BUTTON_SOUTH);
        let mut clicked = Vec::new();
        if submit
            && let Some(entity) = self.focused
            && !self.input_fields.contains_key(&entity)
        {
            self.pending_clicks.push(entity.as_u64());
            clicked.push(entity);
        }
        clicked
    }

    pub fn handle_focus(&mut self, input: &InputState, screen_size: [f32; 2]) -> Vec<EntityId> {
        let focusables = self.focusable_buttons(screen_size);
        let current = self.current_focus_target(&focusables);
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
                let next = step_focus(&focusables, current, step);
                self.set_focused_button(Some(next));
                let _ = input.flush_queued_chars();
                return Vec::new();
            }
        }

        if let Some(focused_id) = self.focused
            && self.input_fields.contains_key(&focused_id)
        {
            self.handle_input_field_editing(focused_id, input);
            return Vec::new();
        }

        let _ = input.flush_queued_chars();

        if !focusables.is_empty()
            && let Some(direction) =
                navigation_direction(input, [prev_x, prev_y], [horizontal, vertical])
        {
            let next = self.navigate_direction(&focusables, current, direction, screen_size);
            self.set_focused_button(Some(next));
        }

        self.collect_submit_click(input)
    }

    fn begin_field_edit(&mut self, entity: EntityId) -> bool {
        let Some(field) = self.input_fields.get_mut(&entity) else {
            return false;
        };
        if field.editing_original_text.is_none() {
            field.editing_original_text = Some(field.text.clone());
            field.was_canceled = false;
        }
        field.caret_position =
            UiInputFieldComponent::clamp_caret(&field.text, field.caret_position);
        true
    }

    fn insert_field_chars(&mut self, entity: EntityId, chars: &[char]) {
        let Some(field) = self.input_fields.get_mut(&entity) else {
            return;
        };
        for &ch in chars {
            if field.character_limit == 0 || field.text.chars().count() < field.character_limit {
                field.text.insert(field.caret_position, ch);
                field.caret_position += ch.len_utf8();
                self.pending_value_changed
                    .push(InputFieldValueChangedEvent {
                        entity,
                        text: field.text.clone(),
                    });
            }
        }
    }

    fn delete_field_chars(&mut self, entity: EntityId, input: &InputState) {
        let Some(field) = self.input_fields.get_mut(&entity) else {
            return;
        };
        if input.is_key_pressed(KEY_BACKSPACE) && field.caret_position > 0 {
            let prev = prev_char_boundary(&field.text, field.caret_position);
            field.text.replace_range(prev..field.caret_position, "");
            field.caret_position = prev;
            self.pending_value_changed
                .push(InputFieldValueChangedEvent {
                    entity,
                    text: field.text.clone(),
                });
        }

        if input.is_key_pressed(KEY_DELETE) && field.caret_position < field.text.len() {
            let next = next_char_boundary(&field.text, field.caret_position);
            field.text.replace_range(field.caret_position..next, "");
            self.pending_value_changed
                .push(InputFieldValueChangedEvent {
                    entity,
                    text: field.text.clone(),
                });
        }
    }

    fn move_field_caret(&mut self, entity: EntityId, input: &InputState) {
        let Some(field) = self.input_fields.get_mut(&entity) else {
            return;
        };
        if input.is_key_pressed(KEY_LEFT_ARROW) && field.caret_position > 0 {
            field.caret_position = prev_char_boundary(&field.text, field.caret_position);
        }
        if input.is_key_pressed(KEY_RIGHT_ARROW) && field.caret_position < field.text.len() {
            field.caret_position = next_char_boundary(&field.text, field.caret_position);
        }
        if input.is_key_pressed(KEY_HOME) {
            field.caret_position = 0;
        }
        if input.is_key_pressed(KEY_END) {
            field.caret_position = field.text.len();
        }
    }

    fn end_field_edit(&mut self, entity: EntityId, input: &InputState) {
        let Some(field) = self.input_fields.get_mut(&entity) else {
            return;
        };
        if input.is_key_pressed(KEY_RETURN) {
            field.was_canceled = false;
            field.editing_original_text = Some(field.text.clone());
            self.pending_end_edit.push(InputFieldEndEditEvent {
                entity,
                text: field.text.clone(),
                canceled: false,
            });
        } else if input.is_key_pressed(KEY_ESCAPE) {
            field.was_canceled = true;
            if let Some(original) = field.editing_original_text.clone()
                && field.text != original
            {
                field.text = original;
                field.caret_position = field.text.len();
                self.pending_value_changed
                    .push(InputFieldValueChangedEvent {
                        entity,
                        text: field.text.clone(),
                    });
            }
            self.pending_end_edit.push(InputFieldEndEditEvent {
                entity,
                text: field.text.clone(),
                canceled: true,
            });
        }
    }

    fn handle_input_field_editing(&mut self, entity: EntityId, input: &InputState) {
        let chars = input.flush_queued_chars();
        if !self.begin_field_edit(entity) {
            return;
        }
        self.insert_field_chars(entity, &chars);
        self.delete_field_chars(entity, input);
        self.move_field_caret(entity, input);
        self.end_field_edit(entity, input);
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
        let resolved_rects = self.resolve_all_element_rects(screen_size, texts);
        for element in &self.binding.elements {
            let (rect_px, scale) = match resolved_rects.get(&element.entity()).copied() {
                Some(resolved) => resolved,
                None => continue,
            };
            if !self.is_hierarchy_visible(element.entity()) {
                continue;
            }
            self.push_element_draw_items(element, rect_px, scale, texts, &mut items);
        }
        let focused_id = self.focused_button(screen_size);
        for &entity in &self.input_field_order {
            let Some(field) = self.input_fields.get(&entity) else {
                continue;
            };
            if !self.is_element_enabled(entity, field.enabled) || !self.is_hierarchy_visible(entity)
            {
                continue;
            }
            let Some((rect_px, scale)) = resolved_rects.get(&entity).copied() else {
                continue;
            };
            self.push_input_field_draw_items(
                field,
                rect_px,
                scale,
                focused_id == Some(entity),
                &mut items,
            );
        }
        self.push_focus_border_items(focused_id, &resolved_rects, &mut items);
        items
    }

    fn push_element_draw_items(
        &self,
        element: &UiElementComponent,
        rect_px: [f32; 4],
        scale: f32,
        texts: &HashMap<EntityId, TextOverride>,
        items: &mut Vec<UiDrawItem>,
    ) {
        match element {
            UiElementComponent::Image(image) => {
                self.push_image_draw_items(image, rect_px, scale, items);
            }
            UiElementComponent::Text(text) => {
                self.push_text_draw_items(text, rect_px, scale, texts, items);
            }
            UiElementComponent::Button(_) => {}
        }
    }

    fn push_image_draw_items(
        &self,
        image: &UiImageComponent,
        rect_px: [f32; 4],
        scale: f32,
        items: &mut Vec<UiDrawItem>,
    ) {
        if !self.is_element_enabled(image.entity, image.enabled) {
            return;
        }
        let sprite_guid = image.sprite_guid.clone();
        let sprite_size = sprite_guid.as_deref().and_then(ui_sprite_size);
        let quads = match (image.draw_mode, sprite_size) {
            (SpriteDrawMode::Sliced, Some(size)) => {
                slice_sprite_quads(image.sprite_border, size, rect_px, FULL_UV, scale)
            }
            (SpriteDrawMode::Tiled, Some(size)) => tile_sprite_quads(size, rect_px, FULL_UV, scale),
            _ => vec![SpriteQuad {
                rect: rect_px,
                uv: FULL_UV,
            }],
        };
        for quad in quads {
            items.push(UiDrawItem {
                rect_px: quad.rect,
                color: image.color,
                sprite: sprite_guid.as_ref().map(|guid| SpriteRef {
                    texture_guid: guid.clone(),
                    uv_rect: quad.uv,
                }),
                text: None,
                text_layout: TextLayoutConfig::default(),
                text_clip_rect: None,
            });
        }
    }

    fn push_text_draw_items(
        &self,
        text: &UiTextComponent,
        rect_px: [f32; 4],
        scale: f32,
        texts: &HashMap<EntityId, TextOverride>,
        items: &mut Vec<UiDrawItem>,
    ) {
        if !self.is_element_enabled(text.entity, text.enabled) {
            return;
        }
        let state = texts.get(&text.entity);
        let content = state.map_or(text.text.as_str(), |state| state.text.as_str());
        let color = state.map_or(text.color, |state| state.color);
        let font_size = state.map_or(text.font_size, |state| state.font_size);
        let text_layout = state.map_or_else(TextLayoutConfig::default, |s| s.layout);
        let text_clip_rect =
            (text_layout.vertical_overflow == VerticalWrapMode::Truncate).then_some(rect_px);
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
            text_layout,
            text_clip_rect,
        });
    }

    fn push_focus_border_items(
        &self,
        focused_id: Option<EntityId>,
        resolved_rects: &HashMap<EntityId, ([f32; 4], f32)>,
        items: &mut Vec<UiDrawItem>,
    ) {
        let Some(focused_id) = focused_id else {
            return;
        };
        let Some((rect_px, scale)) = resolved_rects.get(&focused_id).copied() else {
            return;
        };
        let thickness = (FOCUS_BORDER_THICKNESS * scale).round().max(1.0);
        for edge_rect in focus_border_rects(rect_px, thickness) {
            items.push(UiDrawItem {
                rect_px: edge_rect,
                color: FOCUS_INDICATOR_COLOR,
                sprite: None,
                text: None,
                text_layout: TextLayoutConfig::default(),
                text_clip_rect: None,
            });
        }
    }

    fn push_input_field_draw_items(
        &self,
        field: &UiInputFieldComponent,
        rect_px: [f32; 4],
        scale: f32,
        is_focused: bool,
        items: &mut Vec<UiDrawItem>,
    ) {
        let has_image =
            self.binding.elements.iter().any(
                |el| matches!(el, UiElementComponent::Image(img) if img.entity == field.entity),
            );
        if !has_image {
            items.push(UiDrawItem {
                rect_px,
                color: field.background_color,
                sprite: None,
                text: None,
                text_layout: TextLayoutConfig::default(),
                text_clip_rect: None,
            });
        }
        let pad_x = (6.0 * scale).min(rect_px[2] * 0.25);
        let pad_y = (4.0 * scale).min(rect_px[3] * 0.2);
        let text_rect = [
            rect_px[0] + pad_x,
            rect_px[1] + pad_y,
            (rect_px[2] - 2.0 * pad_x).max(0.0),
            (rect_px[3] - 2.0 * pad_y).max(0.0),
        ];
        let font_px = (field.font_size * scale).max(1.0);
        let (display_text, text_color) = if !field.text.is_empty() {
            (field.text.as_str(), field.color)
        } else {
            (field.placeholder.as_str(), field.placeholder_color)
        };
        if !display_text.is_empty() {
            items.push(UiDrawItem {
                rect_px: text_rect,
                color: text_color,
                sprite: None,
                text: Some(UiTextItem {
                    text: display_text.to_string(),
                    x: text_rect[0],
                    y: text_rect[1],
                    size_px: font_px,
                    color: text_color,
                }),
                text_layout: TextLayoutConfig::default(),
                text_clip_rect: Some(rect_px),
            });
        }
        if is_focused {
            push_input_field_caret_item(field, rect_px, text_rect, font_px, scale, items);
        }
    }

    fn is_input_field_focusable(
        &self,
        field: &UiInputFieldComponent,
        screen_size: [f32; 2],
    ) -> bool {
        if !field.interactable || !self.is_element_enabled(field.entity, field.enabled) {
            return false;
        }
        let Some((rect_px, _)) = self.element_rect(field.entity, screen_size) else {
            return false;
        };
        if rect_px[2] <= 0.0 || rect_px[3] <= 0.0 {
            return false;
        }
        self.is_hierarchy_visible(field.entity)
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
                2 * i32::from(direction[0] > 0.0 || direction[1] > 0.0) - 1,
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
        let fallback_step = 2 * i32::from(direction[0] > 0.0 || direction[1] > 0.0) - 1;
        step_focus(focusables, Some(current_id), fallback_step)
    }

    pub fn element_rect(&self, entity: EntityId, screen_size: [f32; 2]) -> Option<([f32; 4], f32)> {
        self.element_rect_with(entity, screen_size, &HashMap::new())
    }

    pub fn element_rect_with(
        &self,
        entity: EntityId,
        screen_size: [f32; 2],
        texts: &HashMap<EntityId, TextOverride>,
    ) -> Option<([f32; 4], f32)> {
        if screen_size[0] <= 0.0 || screen_size[1] <= 0.0 {
            return None;
        }
        let resolved = self.resolve_all_element_rects(screen_size, texts);
        resolved.get(&entity).copied()
    }

    fn resolve_all_element_rects(
        &self,
        screen_size: [f32; 2],
        texts: &HashMap<EntityId, TextOverride>,
    ) -> HashMap<EntityId, ([f32; 4], f32)> {
        let mut results = HashMap::new();
        if screen_size[0] <= 0.0 || screen_size[1] <= 0.0 {
            return results;
        }
        let pref_sizes = self.compute_preferred_sizes(texts);
        for canvas in &self.binding.canvases {
            if canvas.render_mode != CanvasRenderMode::ScreenSpaceOverlay {
                continue;
            }
            let scale = canvas_scale_factor(canvas, screen_size);
            if scale <= 0.0 {
                continue;
            }
            let canvas_size = [screen_size[0] / scale, screen_size[1] / scale];
            let root_rect = CanvasRect {
                min: [0.0, 0.0],
                size: canvas_size,
            };
            let ctx = SubtreeLayoutContext {
                canvas_size,
                scale,
                pref_sizes: &pref_sizes,
            };
            self.resolve_subtree_rects(canvas.entity, root_rect, &ctx, &mut results, 0);
        }
        results
    }

    fn resolve_subtree_rects(
        &self,
        entity: EntityId,
        current_rect: CanvasRect,
        ctx: &SubtreeLayoutContext<'_>,
        results: &mut HashMap<EntityId, ([f32; 4], f32)>,
        depth: usize,
    ) {
        if depth >= MAX_HIERARCHY_DEPTH {
            return;
        }
        results.insert(
            entity,
            (
                canvas_rect_to_pixels(current_rect, ctx.canvas_size, ctx.scale),
                ctx.scale,
            ),
        );
        if let Some(group) = self.active_layout_group(entity) {
            let children = self.active_children_of(entity);
            if children.is_empty() {
                return;
            }
            let child_rects = resolve_layout_group_children(
                &group,
                current_rect,
                &children,
                &self.rects,
                ctx.pref_sizes,
            );
            for (child, child_rect) in children.into_iter().zip(child_rects) {
                self.resolve_subtree_rects(child, child_rect, ctx, results, depth + 1);
            }
            return;
        }
        let children = self.children_of(entity);
        if children.is_empty() {
            return;
        }
        for child in children {
            let Some(transform) = self.rects.get(&child) else {
                continue;
            };
            let mut child_rect = resolve_canvas_rect(transform, current_rect);
            if let Some(fitter) = self.active_content_size_fitter(child) {
                let pref = ctx
                    .pref_sizes
                    .get(&child)
                    .copied()
                    .unwrap_or(transform.size_delta);
                child_rect = apply_content_size_fitter(&fitter, transform, child_rect, pref);
            }
            self.resolve_subtree_rects(child, child_rect, ctx, results, depth + 1);
        }
    }

    fn compute_preferred_sizes(
        &self,
        texts: &HashMap<EntityId, TextOverride>,
    ) -> HashMap<EntityId, [f32; 2]> {
        let mut sizes = HashMap::new();
        for canvas in &self.binding.canvases {
            self.compute_node_preferred_size(canvas.entity, texts, &mut sizes, 0);
        }
        for rect in &self.binding.rect_transforms {
            if !sizes.contains_key(&rect.entity) {
                self.compute_node_preferred_size(rect.entity, texts, &mut sizes, 0);
            }
        }
        sizes
    }

    fn compute_node_preferred_size(
        &self,
        entity: EntityId,
        texts: &HashMap<EntityId, TextOverride>,
        sizes: &mut HashMap<EntityId, [f32; 2]>,
        depth: usize,
    ) -> [f32; 2] {
        if let Some(&cached) = sizes.get(&entity) {
            return cached;
        }
        if depth >= MAX_HIERARCHY_DEPTH {
            return [0.0, 0.0];
        }
        let children = self.active_children_of(entity);
        let mut child_prefs = Vec::with_capacity(children.len());
        for &child in &children {
            child_prefs.push(self.compute_node_preferred_size(child, texts, sizes, depth + 1));
        }
        let size = if let Some(group) = self.active_layout_group(entity) {
            layout_group_preferred_size(&group, &child_prefs)
        } else if let Some(text_pref) = self.text_preferred_size(entity, texts) {
            text_pref
        } else {
            self.rects.get(&entity).map_or([0.0, 0.0], |r| {
                [r.size_delta[0].max(0.0), r.size_delta[1].max(0.0)]
            })
        };
        sizes.insert(entity, size);
        size
    }

    fn text_preferred_size(
        &self,
        entity: EntityId,
        texts: &HashMap<EntityId, TextOverride>,
    ) -> Option<[f32; 2]> {
        for element in &self.binding.elements {
            let UiElementComponent::Text(text) = element else {
                continue;
            };
            if text.entity != entity || !self.is_element_enabled(text.entity, text.enabled) {
                continue;
            }
            let state = texts.get(&text.entity);
            let content = state.map_or(text.text.as_str(), |s| s.text.as_str());
            let font_size = state.map_or(text.font_size, |s| s.font_size);
            let color = state.map_or(text.color, |s| s.color);
            return Some(measure_text_preferred(content, font_size, color));
        }
        None
    }

    fn children_of(&self, parent: EntityId) -> Vec<EntityId> {
        let mut children = Vec::new();
        for rect in &self.binding.rect_transforms {
            if rect.parent == Some(parent) {
                children.push(rect.entity);
            }
        }
        children
    }

    fn active_children_of(&self, parent: EntityId) -> Vec<EntityId> {
        let mut children = Vec::new();
        for rect in &self.binding.rect_transforms {
            if rect.parent == Some(parent) && self.is_node_active(rect.entity) {
                children.push(rect.entity);
            }
        }
        children
    }

    fn is_node_active(&self, entity: EntityId) -> bool {
        if let Some(&enabled) = self.element_enabled.get(&entity) {
            return enabled;
        }
        for element in &self.binding.elements {
            match element {
                UiElementComponent::Image(img) if img.entity == entity => return img.enabled,
                UiElementComponent::Text(txt) if txt.entity == entity => return txt.enabled,
                _ => {}
            }
        }
        true
    }

    fn active_layout_group(&self, entity: EntityId) -> Option<UiLayoutGroupComponent> {
        if let Ok(guard) = self.layout_group_overrides.read()
            && let Some(group) = guard.get(&entity)
        {
            return self
                .is_element_enabled(group.entity, group.enabled)
                .then(|| group.clone());
        }
        self.binding
            .layout_groups
            .iter()
            .find(|g| g.entity == entity && self.is_element_enabled(g.entity, g.enabled))
            .cloned()
    }

    fn active_content_size_fitter(&self, entity: EntityId) -> Option<UiContentSizeFitterComponent> {
        if let Ok(guard) = self.fitter_overrides.read()
            && let Some(fitter) = guard.get(&entity)
        {
            return self
                .is_element_enabled(fitter.entity, fitter.enabled)
                .then(|| fitter.clone());
        }
        self.binding
            .content_size_fitters
            .iter()
            .find(|f| f.entity == entity && self.is_element_enabled(f.entity, f.enabled))
            .cloned()
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

fn push_input_field_caret_item(
    field: &UiInputFieldComponent,
    rect_px: [f32; 4],
    text_rect: [f32; 4],
    font_px: f32,
    scale: f32,
    items: &mut Vec<UiDrawItem>,
) {
    let caret_idx = UiInputFieldComponent::clamp_caret(&field.text, field.caret_position);
    let prefix = &field.text[..caret_idx];
    let prefix_width = measure_text_preferred(prefix, font_px, field.color)[0];
    let caret_w = (2.0 * scale).round().max(2.0);
    let caret_h = font_px.min(text_rect[3]).max(4.0);
    let max_x = (text_rect[0] + text_rect[2] - caret_w).max(text_rect[0]);
    let caret_x = (text_rect[0] + prefix_width).min(max_x);
    let caret_y = text_rect[1];
    items.push(UiDrawItem {
        rect_px: [caret_x, caret_y, caret_w, caret_h],
        color: field.color,
        sprite: None,
        text: None,
        text_layout: TextLayoutConfig::default(),
        text_clip_rect: Some(rect_px),
    });
}

fn step_focus(focusables: &[EntityId], current: Option<EntityId>, step: i32) -> EntityId {
    let index = current
        .and_then(|id| focusables.iter().position(|&candidate| candidate == id))
        .unwrap_or(0);
    let next = (index as i32 + step).rem_euclid(focusables.len() as i32) as usize;
    focusables[next]
}

fn prev_char_boundary(text: &str, pos: usize) -> usize {
    let mut idx = pos.min(text.len());
    if idx == 0 {
        return 0;
    }
    idx -= 1;
    while idx > 0 && !text.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

fn next_char_boundary(text: &str, pos: usize) -> usize {
    let mut idx = pos.min(text.len());
    if idx >= text.len() {
        return text.len();
    }
    idx += 1;
    while idx < text.len() && !text.is_char_boundary(idx) {
        idx += 1;
    }
    idx
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

struct SubtreeLayoutContext<'a> {
    canvas_size: [f32; 2],
    scale: f32,
    pref_sizes: &'a HashMap<EntityId, [f32; 2]>,
}

fn measure_text_preferred(content: &str, font_size: f32, color: [f32; 4]) -> [f32; 2] {
    static FONTS: OnceLock<Mutex<FontLibrary>> = OnceLock::new();
    if content.is_empty() || font_size <= 0.0 {
        return [0.0, 0.0];
    }
    let mutex = FONTS.get_or_init(|| {
        let mut lib = FontLibrary::new();
        lib.load_bundled_font();
        Mutex::new(lib)
    });
    let Ok(mut fonts) = mutex.lock() else {
        return [content.len() as f32 * font_size * 0.6, font_size];
    };
    let style = TextStyle::new(font_size, font_size, color);
    let layout = layout_text(&mut fonts, content, None, &style);
    [layout.width, layout.height.max(font_size)]
}

fn layout_group_preferred_size(
    group: &UiLayoutGroupComponent,
    child_prefs: &[[f32; 2]],
) -> [f32; 2] {
    let pad_h = (group.padding.left + group.padding.right) as f32;
    let pad_v = (group.padding.top + group.padding.bottom) as f32;
    if child_prefs.is_empty() {
        return [pad_h.max(0.0), pad_v.max(0.0)];
    }
    let total_spacing = group.spacing * (child_prefs.len().saturating_sub(1) as f32);
    match group.axis {
        LayoutGroupAxis::Horizontal => {
            let sum_w: f32 = child_prefs.iter().map(|p| p[0]).sum();
            let max_h: f32 = child_prefs.iter().map(|p| p[1]).fold(0.0, f32::max);
            [
                (pad_h + sum_w + total_spacing).max(0.0),
                (pad_v + max_h).max(0.0),
            ]
        }
        LayoutGroupAxis::Vertical => {
            let max_w: f32 = child_prefs.iter().map(|p| p[0]).fold(0.0, f32::max);
            let sum_h: f32 = child_prefs.iter().map(|p| p[1]).sum();
            [
                (pad_h + max_w).max(0.0),
                (pad_v + sum_h + total_spacing).max(0.0),
            ]
        }
    }
}

fn apply_content_size_fitter(
    fitter: &UiContentSizeFitterComponent,
    transform: &RectTransformComponent,
    mut rect: CanvasRect,
    pref: [f32; 2],
) -> CanvasRect {
    if matches!(
        fitter.horizontal_fit,
        ContentSizeFitMode::MinSize | ContentSizeFitMode::PreferredSize
    ) {
        let old_w = rect.size[0];
        let new_w = pref[0].max(0.0);
        rect.min[0] += (old_w - new_w) * transform.pivot[0];
        rect.size[0] = new_w;
    }
    if matches!(
        fitter.vertical_fit,
        ContentSizeFitMode::MinSize | ContentSizeFitMode::PreferredSize
    ) {
        let old_h = rect.size[1];
        let new_h = pref[1].max(0.0);
        rect.min[1] += (old_h - new_h) * transform.pivot[1];
        rect.size[1] = new_h;
    }
    rect
}

fn alignment_fractions(alignment: u32) -> (f32, f32) {
    let anchor = TextAnchor::from_u32(alignment).unwrap_or(TextAnchor::UpperLeft);
    let h = match anchor.horizontal() {
        HorizontalAlign::Left => 0.0,
        HorizontalAlign::Center => 0.5,
        HorizontalAlign::Right => 1.0,
    };
    let v = match anchor.vertical() {
        VerticalAlign::Top => 0.0,
        VerticalAlign::Middle => 0.5,
        VerticalAlign::Bottom => 1.0,
    };
    (h, v)
}

fn resolve_layout_group_children(
    group: &UiLayoutGroupComponent,
    parent_rect: CanvasRect,
    children: &[EntityId],
    rects: &HashMap<EntityId, RectTransformComponent>,
    pref_sizes: &HashMap<EntityId, [f32; 2]>,
) -> Vec<CanvasRect> {
    let count = children.len();
    if count == 0 {
        return Vec::new();
    }
    let pad_l = group.padding.left as f32;
    let pad_r = group.padding.right as f32;
    let pad_t = group.padding.top as f32;
    let pad_b = group.padding.bottom as f32;
    let inner_w = (parent_rect.size[0] - pad_l - pad_r).max(0.0);
    let inner_h = (parent_rect.size[1] - pad_t - pad_b).max(0.0);
    let base_sizes: Vec<[f32; 2]> = children
        .iter()
        .map(|id| {
            let raw = rects.get(id).map_or([0.0, 0.0], |r| r.size_delta);
            let pref = pref_sizes.get(id).copied().unwrap_or(raw);
            [
                if group.child_control_width {
                    pref[0]
                } else {
                    raw[0]
                }
                .max(0.0),
                if group.child_control_height {
                    pref[1]
                } else {
                    raw[1]
                }
                .max(0.0),
            ]
        })
        .collect();
    let (align_x, align_y) = alignment_fractions(group.child_alignment);
    match group.axis {
        LayoutGroupAxis::Horizontal => layout_horizontal_children(
            group,
            parent_rect,
            inner_w,
            inner_h,
            &base_sizes,
            (align_x, align_y),
        ),
        LayoutGroupAxis::Vertical => layout_vertical_children(
            group,
            parent_rect,
            inner_w,
            inner_h,
            &base_sizes,
            (align_x, align_y),
        ),
    }
}

fn layout_horizontal_children(
    group: &UiLayoutGroupComponent,
    parent_rect: CanvasRect,
    inner_w: f32,
    inner_h: f32,
    base_sizes: &[[f32; 2]],
    align: (f32, f32),
) -> Vec<CanvasRect> {
    let count = base_sizes.len();
    let total_spacing = group.spacing * (count.saturating_sub(1) as f32);
    let sum_w: f32 = base_sizes.iter().map(|s| s[0]).sum();
    let extra_w = (inner_w - (sum_w + total_spacing)).max(0.0);
    let expand_add = if group.child_force_expand_width {
        extra_w / (count as f32)
    } else {
        0.0
    };
    let start_offset_x = if group.child_force_expand_width {
        0.0
    } else {
        extra_w * align.0
    };
    let mut cursor_x = parent_rect.min[0] + (group.padding.left as f32) + start_offset_x;
    let top_y = parent_rect.min[1] + parent_rect.size[1] - (group.padding.top as f32);
    let mut out = Vec::with_capacity(count);
    for base in base_sizes {
        let w = if group.child_control_width {
            base[0] + expand_add
        } else {
            base[0]
        };
        let h = if group.child_control_height && group.child_force_expand_height {
            inner_h
        } else {
            base[1]
        };
        let slack_y = (inner_h - h).max(0.0);
        let child_top_y = top_y - slack_y * align.1;
        out.push(CanvasRect {
            min: [cursor_x, child_top_y - h],
            size: [w, h],
        });
        cursor_x += w + group.spacing;
    }
    out
}

fn layout_vertical_children(
    group: &UiLayoutGroupComponent,
    parent_rect: CanvasRect,
    inner_w: f32,
    inner_h: f32,
    base_sizes: &[[f32; 2]],
    align: (f32, f32),
) -> Vec<CanvasRect> {
    let count = base_sizes.len();
    let total_spacing = group.spacing * (count.saturating_sub(1) as f32);
    let sum_h: f32 = base_sizes.iter().map(|s| s[1]).sum();
    let extra_h = (inner_h - (sum_h + total_spacing)).max(0.0);
    let expand_add = if group.child_force_expand_height {
        extra_h / (count as f32)
    } else {
        0.0
    };
    let start_offset_y = if group.child_force_expand_height {
        0.0
    } else {
        extra_h * align.1
    };
    let left_x = parent_rect.min[0] + (group.padding.left as f32);
    let mut cursor_top_y =
        parent_rect.min[1] + parent_rect.size[1] - (group.padding.top as f32) - start_offset_y;
    let mut out = Vec::with_capacity(count);
    for base in base_sizes {
        let h = if group.child_control_height {
            base[1] + expand_add
        } else {
            base[1]
        };
        let w = if group.child_control_width && group.child_force_expand_width {
            inner_w
        } else {
            base[0]
        };
        let slack_x = (inner_w - w).max(0.0);
        let child_min_x = left_x + slack_x * align.0;
        out.push(CanvasRect {
            min: [child_min_x, cursor_top_y - h],
            size: [w, h],
        });
        cursor_top_y -= h + group.spacing;
    }
    out
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
        CanvasScalerComponent, SpriteDrawMode, UiButtonComponent, UiImageComponent, UiPadding,
        UiTextComponent,
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

    fn simple_image_element(
        entity: EntityId,
        enabled: bool,
        color: [f32; 4],
    ) -> UiElementComponent {
        UiElementComponent::Image(UiImageComponent {
            entity,
            enabled,
            color,
            sprite_guid: None,
            draw_mode: SpriteDrawMode::Simple,
            sprite_border: [0.0; 4],
        })
    }

    fn button_element(entity: EntityId, interactable: bool) -> UiElementComponent {
        UiElementComponent::Button(UiButtonComponent {
            entity,
            target_graphic: Some(entity),
            interactable,
        })
    }

    fn sample_rect_transforms() -> Vec<RectTransformComponent> {
        vec![
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
        ]
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
            rect_transforms: sample_rect_transforms(),
            elements: vec![
                simple_image_element(TITLE_PANEL_ID, true, [0.08, 0.1, 0.16, 0.88]),
                simple_image_element(PLAY_BUTTON_ID, true, [0.2, 0.65, 0.32, 1.0]),
                button_element(PLAY_BUTTON_ID, true),
                UiElementComponent::Text(UiTextComponent {
                    entity: PLAY_BUTTON_ID,
                    enabled: true,
                    text: "PLAY".to_string(),
                    font_size: 22.0,
                    color: [1.0, 1.0, 1.0, 1.0],
                }),
                simple_image_element(QUIT_BUTTON_ID, true, [0.55, 0.2, 0.22, 1.0]),
                button_element(QUIT_BUTTON_ID, true),
                simple_image_element(DISABLED_BUTTON_ID, true, [0.3, 0.3, 0.3, 1.0]),
                button_element(DISABLED_BUTTON_ID, false),
                simple_image_element(RESULTS_PANEL_ID, false, [0.08, 0.1, 0.16, 0.92]),
                simple_image_element(RETRY_BUTTON_ID, true, [0.2, 0.65, 0.32, 1.0]),
                button_element(RETRY_BUTTON_ID, true),
                simple_image_element(ARENA_SELECT_BUTTON_ID, true, [0.22, 0.45, 0.72, 1.0]),
                button_element(ARENA_SELECT_BUTTON_ID, true),
            ],
            layout_groups: Vec::new(),
            content_size_fitters: Vec::new(),
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

    fn assert_results_panel_navigation(canvas: &mut CanvasState, input: &mut InputState) {
        canvas.set_element_enabled(TITLE_PANEL_ID, false);
        canvas.set_element_enabled(RESULTS_PANEL_ID, true);
        assert_eq!(canvas.focused_button(SCREEN), Some(RETRY_BUTTON_ID));

        input.queue_key_event(KEY_RIGHT_ARROW, true);
        input.begin_frame(1.0 / 60.0);
        canvas.handle_focus(input, SCREEN);
        assert_eq!(canvas.focused_button(SCREEN), Some(ARENA_SELECT_BUTTON_ID));
        input.queue_key_event(KEY_RIGHT_ARROW, false);
        input.begin_frame(1.0 / 60.0);

        input.queue_key_event(KEY_LEFT_ARROW, true);
        input.begin_frame(1.0 / 60.0);
        canvas.handle_focus(input, SCREEN);
        assert_eq!(canvas.focused_button(SCREEN), Some(RETRY_BUTTON_ID));
        input.queue_key_event(KEY_LEFT_ARROW, false);
        input.begin_frame(1.0 / 60.0);

        input.set_gamepad_button(GAMEPAD_BUTTON_DPAD_RIGHT, true);
        input.begin_frame(1.0 / 60.0);
        canvas.handle_focus(input, SCREEN);
        assert_eq!(canvas.focused_button(SCREEN), Some(ARENA_SELECT_BUTTON_ID));
        input.set_gamepad_button(GAMEPAD_BUTTON_DPAD_RIGHT, false);
        input.begin_frame(1.0 / 60.0);
        canvas.handle_focus(input, SCREEN);

        input.set_gamepad_axis(GAMEPAD_AXIS_LEFT_X, -0.85);
        input.begin_frame(1.0 / 60.0);
        canvas.handle_focus(input, SCREEN);
        assert_eq!(canvas.focused_button(SCREEN), Some(RETRY_BUTTON_ID));
        input.set_gamepad_axis(GAMEPAD_AXIS_LEFT_X, 0.0);
        input.begin_frame(1.0 / 60.0);
        canvas.handle_focus(input, SCREEN);

        input.set_gamepad_button(GAMEPAD_BUTTON_DPAD_UP, true);
        input.begin_frame(1.0 / 60.0);
        canvas.handle_focus(input, SCREEN);
        assert_eq!(canvas.focused_button(SCREEN), Some(ARENA_SELECT_BUTTON_ID));
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

        assert_results_panel_navigation(&mut canvas, &mut input);
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

    #[test]
    fn text_override_layout_plumbs_alignment_wrap_and_truncate_clip_into_ui_text() {
        use unitas_render::ui_pass::{GlyphTextPainter, HorizontalAlign, UiElement, VerticalAlign};

        let canvas = sample_canvas();
        let default_items = canvas.draw_items(SCREEN);
        let default_play = default_items
            .iter()
            .find(|item| item.text.is_some())
            .and_then(|item| item.text());
        assert!(default_play.is_some());
        if let Some(ui_text) = default_play {
            assert_eq!(ui_text.horizontal_align, HorizontalAlign::Left);
            assert_eq!(ui_text.vertical_align, VerticalAlign::Top);
            assert!(!ui_text.wrap);
            assert_eq!(ui_text.max_width, None);
            assert_eq!(ui_text.line_spacing, 1.0);
            assert_eq!(ui_text.clip_rect, None);
        }

        let overrides = HashMap::from([(
            PLAY_BUTTON_ID,
            TextOverride {
                text: "START GAME NOW".to_string(),
                font_size: 20.0,
                color: [1.0, 1.0, 1.0, 1.0],
                layout: TextLayoutConfig {
                    alignment: TextAnchor::MiddleCenter,
                    horizontal_overflow: HorizontalWrapMode::Wrap,
                    vertical_overflow: VerticalWrapMode::Truncate,
                    line_spacing: 1.25,
                },
            },
        )]);
        let overridden_items = canvas.draw_items_with(SCREEN, &overrides);
        let play_item = overridden_items.iter().find(|item| item.text.is_some());
        assert!(play_item.is_some());
        if let Some(item) = play_item
            && let Some(ui_text) = item.text()
        {
            assert_eq!(ui_text.horizontal_align, HorizontalAlign::Center);
            assert_eq!(ui_text.vertical_align, VerticalAlign::Middle);
            assert!(ui_text.wrap);
            assert_eq!(ui_text.max_width, Some(item.rect_px[2]));
            assert_eq!(ui_text.line_spacing, 1.25);
            assert_eq!(ui_text.clip_rect, Some(item.rect_px));
        }

        let mut painter = GlyphTextPainter::new();
        let mut image = Rgba8::new(800, 600);
        let mut target = UiTarget {
            image: &mut image,
            ids: None,
        };
        composite_ui(&mut target, &overridden_items, 1, Some(&mut painter));
    }

    fn layout_group_test_binding(
        panel_id: EntityId,
        row_id: EntityId,
        icon_id: EntityId,
        label_id: EntityId,
        footer_id: EntityId,
    ) -> UiBinding {
        UiBinding {
            canvases: vec![CanvasComponent {
                entity: CANVAS_ID,
                render_mode: CanvasRenderMode::ScreenSpaceOverlay,
                sorting_order: 0,
                scaler: None,
            }],
            rect_transforms: vec![
                centered_rect(CANVAS_ID, None, [0.0, 0.0], SCREEN),
                RectTransformComponent {
                    entity: panel_id,
                    parent: Some(CANVAS_ID),
                    anchor_min: [0.0, 1.0],
                    anchor_max: [0.0, 1.0],
                    anchored_position: [50.0, -40.0],
                    size_delta: [10.0, 10.0],
                    pivot: [0.0, 1.0],
                },
                centered_rect(row_id, Some(panel_id), [0.0, 0.0], [10.0, 10.0]),
                centered_rect(icon_id, Some(row_id), [0.0, 0.0], [32.0, 32.0]),
                centered_rect(label_id, Some(row_id), [0.0, 0.0], [80.0, 20.0]),
                centered_rect(footer_id, Some(panel_id), [0.0, 0.0], [100.0, 24.0]),
            ],
            elements: vec![
                simple_image_element(panel_id, true, [0.1, 0.1, 0.1, 1.0]),
                simple_image_element(icon_id, true, [1.0, 0.0, 0.0, 1.0]),
                UiElementComponent::Text(UiTextComponent {
                    entity: label_id,
                    enabled: true,
                    text: "PLAY GAME".to_string(),
                    font_size: 20.0,
                    color: [1.0, 1.0, 1.0, 1.0],
                }),
                simple_image_element(footer_id, true, [0.0, 0.5, 1.0, 1.0]),
            ],
            layout_groups: layout_group_test_groups(panel_id, row_id),
            content_size_fitters: vec![UiContentSizeFitterComponent {
                entity: panel_id,
                horizontal_fit: ContentSizeFitMode::PreferredSize,
                vertical_fit: ContentSizeFitMode::PreferredSize,
                enabled: true,
            }],
        }
    }

    fn layout_group_test_groups(
        panel_id: EntityId,
        row_id: EntityId,
    ) -> Vec<UiLayoutGroupComponent> {
        vec![
            UiLayoutGroupComponent {
                entity: panel_id,
                axis: LayoutGroupAxis::Vertical,
                padding: UiPadding {
                    left: 10,
                    right: 14,
                    top: 8,
                    bottom: 12,
                },
                spacing: 6.0,
                child_alignment: 1,
                child_control_width: true,
                child_control_height: true,
                child_force_expand_width: true,
                child_force_expand_height: false,
                enabled: true,
            },
            UiLayoutGroupComponent {
                entity: row_id,
                axis: LayoutGroupAxis::Horizontal,
                padding: UiPadding {
                    left: 4,
                    right: 4,
                    top: 2,
                    bottom: 2,
                },
                spacing: 8.0,
                child_alignment: 3,
                child_control_width: true,
                child_control_height: false,
                child_force_expand_width: false,
                child_force_expand_height: false,
                enabled: true,
            },
        ]
    }

    fn assert_layout_group_overrides(
        canvas: &CanvasState,
        panel_id: EntityId,
        expected_panel_h: f32,
    ) {
        canvas.set_layout_group(UiLayoutGroupComponent {
            entity: panel_id,
            axis: LayoutGroupAxis::Vertical,
            padding: UiPadding {
                left: 10,
                right: 14,
                top: 8,
                bottom: 12,
            },
            spacing: 20.0,
            child_alignment: 1,
            child_control_width: false,
            child_control_height: true,
            child_force_expand_width: false,
            child_force_expand_height: false,
            enabled: true,
        });
        let (updated_panel, _) = canvas
            .element_rect(panel_id, SCREEN)
            .unwrap_or(([0.0; 4], 1.0));
        assert!((updated_panel[3] - (expected_panel_h + 14.0)).abs() < 0.01);

        canvas.set_content_size_fitter(UiContentSizeFitterComponent {
            entity: panel_id,
            horizontal_fit: ContentSizeFitMode::Unconstrained,
            vertical_fit: ContentSizeFitMode::Unconstrained,
            enabled: true,
        });
        let (unconstrained_panel, _) = canvas
            .element_rect(panel_id, SCREEN)
            .unwrap_or(([0.0; 4], 1.0));
        assert!((unconstrained_panel[2] - 10.0).abs() < 0.01);
        assert!((unconstrained_panel[3] - 10.0).abs() < 0.01);
    }

    #[test]
    fn vertical_and_horizontal_layout_groups_with_content_size_fitter_resolve_unity_child_rects() {
        let panel_id = EntityId::new(200);
        let row_id = EntityId::new(210);
        let icon_id = EntityId::new(211);
        let label_id = EntityId::new(212);
        let footer_id = EntityId::new(220);

        let binding = layout_group_test_binding(panel_id, row_id, icon_id, label_id, footer_id);
        let canvas = CanvasState::new(binding);
        let (label_rect, _) = canvas
            .element_rect(label_id, SCREEN)
            .unwrap_or(([0.0; 4], 1.0));
        assert!(label_rect[2] > 50.0);
        let expected_row_pref_w = 4.0 + 32.0 + 8.0 + label_rect[2] + 4.0;
        let expected_row_pref_h = 2.0 + 32.0_f32.max(label_rect[3]) + 2.0;
        let expected_panel_w = 10.0 + expected_row_pref_w.max(100.0) + 14.0;
        let expected_panel_h = 8.0 + expected_row_pref_h + 6.0 + 24.0 + 12.0;

        let (panel_rect, _) = canvas
            .element_rect(panel_id, SCREEN)
            .unwrap_or(([0.0; 4], 1.0));
        assert!((panel_rect[0] - 50.0).abs() < 0.01);
        assert!((panel_rect[1] - 40.0).abs() < 0.01);
        assert!((panel_rect[2] - expected_panel_w).abs() < 0.01);
        assert!((panel_rect[3] - expected_panel_h).abs() < 0.01);

        let (row_rect, _) = canvas
            .element_rect(row_id, SCREEN)
            .unwrap_or(([0.0; 4], 1.0));
        assert!((row_rect[0] - 60.0).abs() < 0.01);
        assert!((row_rect[1] - 48.0).abs() < 0.01);
        assert!((row_rect[2] - (expected_panel_w - 24.0)).abs() < 0.01);
        assert!((row_rect[3] - expected_row_pref_h).abs() < 0.01);

        let (icon_rect, _) = canvas
            .element_rect(icon_id, SCREEN)
            .unwrap_or(([0.0; 4], 1.0));
        assert!((icon_rect[0] - 64.0).abs() < 0.01);
        assert!((icon_rect[2] - 32.0).abs() < 0.01);
        assert!((icon_rect[3] - 32.0).abs() < 0.01);
        assert!((label_rect[0] - (64.0 + 32.0 + 8.0)).abs() < 0.01);

        let (footer_rect, _) = canvas
            .element_rect(footer_id, SCREEN)
            .unwrap_or(([0.0; 4], 1.0));
        assert!((footer_rect[0] - 60.0).abs() < 0.01);
        assert!((footer_rect[1] - (48.0 + expected_row_pref_h + 6.0)).abs() < 0.01);
        assert!((footer_rect[2] - (expected_panel_w - 24.0)).abs() < 0.01);
        assert!((footer_rect[3] - 24.0).abs() < 0.01);

        assert_layout_group_overrides(&canvas, panel_id, expected_panel_h);
    }

    fn assert_abi_layout_group_and_fitter() {
        let handle = crate::unitas_runtime_new();
        assert_ne!(handle, 0);
        let packed_padding = 6u64 | (8u64 << 16) | (10u64 << 32) | (12u64 << 48);
        let obj_id = crate::unitas_runtime_object_create(handle, 0, 0);
        assert_ne!(obj_id, 0);
        assert_eq!(
            crate::unitas_runtime_ui_layout_group_set(
                handle,
                obj_id,
                1,
                packed_padding,
                4.0,
                1,
                1 | 2 | 4 | 16,
            ),
            0
        );
        assert_eq!(
            crate::unitas_runtime_ui_layout_group_set(handle, obj_id, 9, packed_padding, 4.0, 1, 0),
            -2
        );
        assert_eq!(
            crate::unitas_runtime_ui_content_size_fitter_set(handle, obj_id, 2, 1, 1),
            0
        );
        assert_eq!(
            crate::unitas_runtime_ui_content_size_fitter_set(handle, obj_id, 9, 1, 1),
            -2
        );
        assert_eq!(crate::unitas_runtime_destroy(handle), 0);
    }

    #[test]
    fn runtime_ui_and_abi_configure_layout_groups_and_content_size_fitter() {
        let mut runtime = crate::Runtime::new().unwrap_or_else(|_| std::process::abort());
        let canvas_id = runtime
            .create_runtime_object("HudCanvas")
            .unwrap_or(EntityId::new(1));
        assert!(runtime.set_ui_layout_group(
            canvas_id.as_u64(),
            LayoutGroupAxis::Vertical,
            UiPadding {
                left: 6,
                right: 8,
                top: 10,
                bottom: 12,
            },
            4.0,
            1,
            1 | 2 | 4 | 16,
        ));
        assert!(runtime.set_ui_content_size_fitter(
            canvas_id.as_u64(),
            ContentSizeFitMode::PreferredSize,
            ContentSizeFitMode::MinSize,
            true,
        ));
        assert!(!runtime.set_ui_layout_group(
            0,
            LayoutGroupAxis::Horizontal,
            UiPadding::default(),
            0.0,
            0,
            0,
        ));
        assert!(!runtime.set_ui_content_size_fitter(
            0,
            ContentSizeFitMode::Unconstrained,
            ContentSizeFitMode::Unconstrained,
            false,
        ));
        assert_abi_layout_group_and_fitter();
    }
}
