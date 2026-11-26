use unitas_render::{Camera, Mat4, Mesh, Vec3, look_at};

use crate::input::{
    InputState, KEY_DOWN_ARROW, KEY_ESCAPE, KEY_LEFT_ARROW, KEY_RETURN, KEY_RIGHT_ARROW,
    KEY_UP_ARROW, STATE_PRESSED,
};

/// Color of the always-visible focus border drawn around the focused widget.
pub const FOCUS_INDICATOR_COLOR: [f32; 3] = [1.0, 0.9, 0.0];

const FOCUS_BORDER_THICKNESS: f32 = 2.0;

/// Text produced by [`Ui::text_overlay`] for the render overlay path.
/// Mirrors `unitas_render::TextOverlayItem`, which is only available with the
/// device feature; the host maps these into the overlay verbatim.
#[derive(Clone, Debug, PartialEq)]
pub struct UiTextItem {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub size_px: f32,
    pub color: [f32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchor {
    TopLeft,
    Top,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

impl Anchor {
    fn pivot(self) -> (f32, f32) {
        match self {
            Anchor::TopLeft => (0.0, 0.0),
            Anchor::Top => (0.5, 0.0),
            Anchor::TopRight => (1.0, 0.0),
            Anchor::Left => (0.0, 0.5),
            Anchor::Center => (0.5, 0.5),
            Anchor::Right => (1.0, 0.5),
            Anchor::BottomLeft => (0.0, 1.0),
            Anchor::Bottom => (0.5, 1.0),
            Anchor::BottomRight => (1.0, 1.0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiRect {
    pub anchor: Anchor,
    pub offset: [f32; 2],
    pub size: [f32; 2],
}

impl UiRect {
    pub const fn new(anchor: Anchor, offset: [f32; 2], size: [f32; 2]) -> Self {
        Self {
            anchor,
            offset,
            size,
        }
    }

    pub fn resolve(&self, parent: &Rect) -> Rect {
        let (ax, ay) = self.anchor.pivot();
        Rect {
            x: parent.x + parent.width * ax + self.offset[0],
            y: parent.y + parent.height * ay + self.offset[1],
            width: self.size[0],
            height: self.size[1],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub const fn screen(width: f32, height: f32) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width,
            height,
        }
    }

    pub fn contains(&self, px: f64, py: f64) -> bool {
        px >= f64::from(self.x)
            && py >= f64::from(self.y)
            && px < f64::from(self.x + self.width)
            && py < f64::from(self.y + self.height)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Widget {
    Panel {
        rect: UiRect,
        color: [f32; 3],
    },
    Image {
        rect: UiRect,
        tint: [f32; 3],
    },
    Button {
        id: String,
        rect: UiRect,
        color: [f32; 3],
        hover_color: [f32; 3],
        pressed_color: [f32; 3],
    },
    Label {
        rect: UiRect,
        text: String,
        size_px: f32,
        color: [f32; 4],
    },
}

impl Widget {
    fn rect(&self) -> &UiRect {
        match self {
            Widget::Panel { rect, .. } => rect,
            Widget::Image { rect, .. } => rect,
            Widget::Button { rect, .. } => rect,
            Widget::Label { rect, .. } => rect,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UiEvent {
    Clicked(String),
    Cancel,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UiQuad {
    pub mesh: Mesh,
    pub model: Mat4,
    pub color: [f32; 3],
}

pub struct Ui {
    widgets: Vec<Widget>,
    hovered: Option<usize>,
    active: Option<usize>,
    focus: Option<String>,
    focus_axis_prev: [f64; 2],
}

impl Default for Ui {
    fn default() -> Self {
        Self::new()
    }
}

impl Ui {
    pub fn new() -> Self {
        Self {
            widgets: Vec::new(),
            hovered: None,
            active: None,
            focus: None,
            focus_axis_prev: [0.0; 2],
        }
    }

    pub fn clear(&mut self) {
        self.widgets.clear();
        self.hovered = None;
        self.active = None;
        // `focus` survives the rebuild: it is a button id validated lazily
        // against the current widgets, so immediate-mode callers keep it.
    }

    pub fn panel(&mut self, rect: UiRect, color: [f32; 3]) {
        self.widgets.push(Widget::Panel { rect, color });
    }

    pub fn image(&mut self, rect: UiRect, tint: [f32; 3]) {
        self.widgets.push(Widget::Image { rect, tint });
    }

    pub fn button(
        &mut self,
        id: &str,
        rect: UiRect,
        color: [f32; 3],
        hover_color: [f32; 3],
        pressed_color: [f32; 3],
    ) {
        self.widgets.push(Widget::Button {
            id: id.to_string(),
            rect,
            color,
            hover_color,
            pressed_color,
        });
    }

    pub fn label(&mut self, rect: UiRect, text: &str, size_px: f32, color: [f32; 4]) {
        self.widgets.push(Widget::Label {
            rect,
            text: text.to_string(),
            size_px,
            color,
        });
    }

    /// Resolves label widgets into text items for the render overlay path.
    pub fn text_overlay(&self, screen: &Rect) -> Vec<UiTextItem> {
        self.widgets
            .iter()
            .filter_map(|widget| match widget {
                Widget::Label {
                    rect,
                    text,
                    size_px,
                    color,
                } => {
                    let resolved = rect.resolve(screen);
                    Some(UiTextItem {
                        text: text.clone(),
                        x: resolved.x,
                        y: resolved.y,
                        size_px: *size_px,
                        color: *color,
                    })
                }
                _ => None,
            })
            .collect()
    }

    pub fn focused(&self) -> Option<&str> {
        let order = self.focus_order();
        self.focus
            .as_deref()
            .filter(|id| order.contains(id))
            .or_else(|| order.first().copied())
    }

    fn focus_order(&self) -> Vec<&str> {
        self.widgets
            .iter()
            .filter_map(|widget| match widget {
                Widget::Button { id, .. } => Some(id.as_str()),
                _ => None,
            })
            .collect()
    }

    /// Applies keyboard/gamepad focus navigation for one frame: arrow keys and
    /// the named `Horizontal`/`Vertical` axes move focus (wrapping), the named
    /// `Submit` button (or Return) clicks the focused button, and the named
    /// `Cancel` button (or Escape) emits [`UiEvent::Cancel`].
    pub fn handle_focus(&mut self, input: &InputState) -> Vec<UiEvent> {
        let horizontal = input.axis_raw("Horizontal").unwrap_or(0.0);
        let vertical = input.axis_raw("Vertical").unwrap_or(0.0);
        let mut step = 0i32;
        if input.is_key_pressed(KEY_RIGHT_ARROW)
            || (self.focus_axis_prev[0] <= 0.0 && horizontal > 0.0)
        {
            step = 1;
        } else if input.is_key_pressed(KEY_LEFT_ARROW)
            || (self.focus_axis_prev[0] >= 0.0 && horizontal < 0.0)
        {
            step = -1;
        } else if input.is_key_pressed(KEY_DOWN_ARROW)
            || (self.focus_axis_prev[1] <= 0.0 && vertical > 0.0)
        {
            step = 1;
        } else if input.is_key_pressed(KEY_UP_ARROW)
            || (self.focus_axis_prev[1] >= 0.0 && vertical < 0.0)
        {
            step = -1;
        }
        self.focus_axis_prev = [horizontal, vertical];
        if step != 0 {
            let order = self.focus_order();
            if !order.is_empty() {
                let current = self
                    .focused()
                    .and_then(|id| order.iter().position(|candidate| *candidate == id));
                let next = match current {
                    Some(index) => (index as i32 + step).rem_euclid(order.len() as i32) as usize,
                    None if step > 0 => 0,
                    None => order.len() - 1,
                };
                self.focus = Some(order[next].to_string());
            }
        }
        let mut events = Vec::new();
        let submit = named_button_pressed(input, "Submit") || input.is_key_pressed(KEY_RETURN);
        let cancel = named_button_pressed(input, "Cancel") || input.is_key_pressed(KEY_ESCAPE);
        if submit && let Some(id) = self.focused() {
            events.push(UiEvent::Clicked(id.to_string()));
        }
        if cancel {
            events.push(UiEvent::Cancel);
        }
        events
    }

    pub fn hovered(&self) -> Option<&str> {
        self.hovered.and_then(|index| match &self.widgets[index] {
            Widget::Button { id, .. } => Some(id.as_str()),
            _ => None,
        })
    }

    pub fn handle_pointer(&mut self, input: &InputState, screen: &Rect) -> Vec<UiEvent> {
        let pointer = input.pointer_frame();
        let (px, py) = (f64::from(pointer.position.0), f64::from(pointer.position.1));
        let mut hovered = None;
        for (index, widget) in self.widgets.iter().enumerate().rev() {
            if let Widget::Button { rect, .. } = widget
                && rect.resolve(screen).contains(px, py)
            {
                hovered = Some(index);
                break;
            }
        }
        self.hovered = hovered;
        let mut events = Vec::new();
        if pointer.primary_pressed {
            self.active = hovered;
        }
        if self.active.is_some() && self.active != hovered {
            self.active = None;
        }
        if pointer.primary_released {
            if let (Some(active), Some(hovered)) = (self.active, hovered)
                && active == hovered
                && let Widget::Button { id, .. } = &self.widgets[active]
            {
                events.push(UiEvent::Clicked(id.clone()));
            }
            self.active = None;
        }
        events
    }

    pub fn draw_list(&self, screen: &Rect) -> Vec<UiQuad> {
        let focused_id = self.focused();
        let mut quads = Vec::new();
        for (index, widget) in self.widgets.iter().enumerate() {
            // Labels render through the text overlay path, not as quads.
            if let Widget::Label { .. } = widget {
                continue;
            }
            let rect = widget.rect().resolve(screen);
            let color = match widget {
                Widget::Panel { color, .. } => *color,
                Widget::Image { tint, .. } => *tint,
                Widget::Button {
                    color,
                    hover_color,
                    pressed_color,
                    ..
                } => {
                    if self.active == Some(index) {
                        *pressed_color
                    } else if self.hovered == Some(index) {
                        *hover_color
                    } else {
                        *color
                    }
                }
                Widget::Label { .. } => unreachable!("labels are skipped above"),
            };
            let is_focused = matches!(
                widget,
                Widget::Button { id, .. } if focused_id == Some(id.as_str())
            );
            quads.push(UiQuad {
                mesh: unit_quad(),
                model: quad_model(&rect),
                color,
            });
            if is_focused {
                quads.extend(focus_border_quads(&rect));
            }
        }
        quads
    }
}

fn named_button_pressed(input: &InputState, name: &str) -> bool {
    input
        .button_state(name)
        .is_some_and(|state| state & STATE_PRESSED != 0)
}

/// Four thin edge quads forming an always-visible border around `rect`.
fn focus_border_quads(rect: &Rect) -> Vec<UiQuad> {
    let thickness = FOCUS_BORDER_THICKNESS.min(rect.width * 0.5).max(1.0);
    let edges = [
        Rect {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: thickness,
        },
        Rect {
            x: rect.x,
            y: rect.y + rect.height - thickness,
            width: rect.width,
            height: thickness,
        },
        Rect {
            x: rect.x,
            y: rect.y,
            width: thickness,
            height: rect.height,
        },
        Rect {
            x: rect.x + rect.width - thickness,
            y: rect.y,
            width: thickness,
            height: rect.height,
        },
    ];
    edges
        .iter()
        .map(|edge| UiQuad {
            mesh: unit_quad(),
            model: quad_model(edge),
            color: FOCUS_INDICATOR_COLOR,
        })
        .collect()
}

pub fn ui_quad_mesh() -> Mesh {
    unit_quad()
}

pub fn quad_model(rect: &Rect) -> Mat4 {
    let mut model = Mat4::identity();
    model.set(0, 0, rect.width);
    model.set(1, 1, rect.height);
    model.set(3, 0, rect.x);
    model.set(3, 1, rect.y);
    model
}

pub fn ui_camera(screen_width: f32, screen_height: f32) -> Camera {
    let height = screen_height.max(1.0);
    Camera::orthographic(
        (height * 0.5).max(0.001),
        screen_width.max(1.0) / height,
        0.001,
        2.0,
    )
}

pub fn ui_view(screen_width: f32, screen_height: f32) -> Mat4 {
    let center = Vec3::new(screen_width * 0.5, screen_height * 0.5, 0.0);
    look_at(
        Vec3::new(center.x, center.y, 1.0),
        center,
        Vec3::new(0.0, -1.0, 0.0),
    )
}

fn unit_quad() -> Mesh {
    Mesh {
        positions: vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ],
        normals: vec![Vec3::new(0.0, 0.0, 1.0); 4],
        indices: vec![0, 1, 2, 0, 2, 3],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{
        AxisConfig, InputState, KEY_DOWN_ARROW, KEY_ESCAPE, KEY_LEFT_ARROW, KEY_RETURN,
        KEY_RIGHT_ARROW, KEY_SPACE, KEY_UP_ARROW, PRIMARY_MOUSE_BUTTON,
    };

    fn press(input: &mut InputState, button: u32, pressed: bool) {
        input.queue_mouse_button_event(button, pressed);
        input.begin_frame(1.0 / 60.0);
    }

    fn move_to(input: &mut InputState, x: f64, y: f64) {
        input.queue_pointer_move(x, y);
        input.begin_frame(1.0 / 60.0);
    }

    fn press_key(input: &mut InputState, key: u32) {
        input.queue_key_event(key, true);
        input.begin_frame(1.0 / 60.0);
    }

    fn release_key(input: &mut InputState, key: u32) {
        input.queue_key_event(key, false);
        input.begin_frame(1.0 / 60.0);
    }

    fn button_axis(name: &str, positive_button: &str) -> AxisConfig {
        AxisConfig {
            name: name.to_string(),
            negative_button: String::new(),
            positive_button: positive_button.to_string(),
            alt_negative_button: String::new(),
            alt_positive_button: String::new(),
            gravity: 1000.0,
            dead: 0.001,
            sensitivity: 1000.0,
            snap: false,
            invert: false,
            axis_type: 0,
            axis: 0,
        }
    }

    fn three_button_ui() -> Ui {
        let mut ui = Ui::new();
        for (id, x) in [("alpha", -120.0), ("bravo", 0.0), ("charlie", 120.0)] {
            ui.button(
                id,
                UiRect::new(Anchor::Center, [x, -20.0], [100.0, 40.0]),
                [0.2; 3],
                [0.3; 3],
                [0.4; 3],
            );
        }
        ui
    }

    #[test]
    fn anchored_rect_resolves_against_screen() {
        let screen = Rect::screen(800.0, 600.0);
        let top_right = UiRect::new(Anchor::TopRight, [-100.0, 20.0], [50.0, 30.0]);
        assert_eq!(
            top_right.resolve(&screen),
            Rect {
                x: 700.0,
                y: 20.0,
                width: 50.0,
                height: 30.0,
            }
        );
        let center = UiRect::new(Anchor::Center, [-25.0, -15.0], [50.0, 30.0]);
        assert_eq!(
            center.resolve(&screen),
            Rect {
                x: 375.0,
                y: 285.0,
                width: 50.0,
                height: 30.0,
            }
        );
        let bottom_left = UiRect::new(Anchor::BottomLeft, [10.0, -40.0], [50.0, 30.0]);
        assert_eq!(
            bottom_left.resolve(&screen),
            Rect {
                x: 10.0,
                y: 560.0,
                width: 50.0,
                height: 30.0,
            }
        );
    }

    #[test]
    fn all_nine_anchors_resolve_and_hit_test() {
        let screen = Rect::screen(800.0, 600.0);
        let cases = [
            (Anchor::TopLeft, 0.0, 0.0),
            (Anchor::Top, 400.0, 0.0),
            (Anchor::TopRight, 800.0, 0.0),
            (Anchor::Left, 0.0, 300.0),
            (Anchor::Center, 400.0, 300.0),
            (Anchor::Right, 800.0, 300.0),
            (Anchor::BottomLeft, 0.0, 600.0),
            (Anchor::Bottom, 400.0, 600.0),
            (Anchor::BottomRight, 800.0, 600.0),
        ];
        for (anchor, x, y) in cases {
            let rect = UiRect::new(anchor, [0.0, 0.0], [40.0, 20.0]);
            let resolved = rect.resolve(&screen);
            assert_eq!((resolved.x, resolved.y), (x, y));
            let mut ui = Ui::new();
            ui.button("probe", rect, [0.1; 3], [0.2; 3], [0.3; 3]);
            let mut input = InputState::new();
            move_to(&mut input, f64::from(x) + 1.0, f64::from(y) + 1.0);
            ui.handle_pointer(&input, &screen);
            assert_eq!(ui.hovered(), Some("probe"));
        }
    }

    #[test]
    fn anchored_rect_nests_inside_parent_rect() {
        let screen = Rect::screen(800.0, 600.0);
        let panel = UiRect::new(Anchor::Center, [-100.0, -75.0], [200.0, 150.0]);
        let panel_rect = panel.resolve(&screen);
        let child = UiRect::new(Anchor::Bottom, [-20.0, -30.0], [40.0, 20.0]);
        assert_eq!(
            child.resolve(&panel_rect),
            Rect {
                x: 380.0,
                y: 345.0,
                width: 40.0,
                height: 20.0,
            }
        );
    }

    #[test]
    fn rect_contains_is_half_open() {
        let rect = Rect {
            x: 10.0,
            y: 20.0,
            width: 30.0,
            height: 40.0,
        };
        assert!(rect.contains(10.0, 20.0));
        assert!(rect.contains(39.9, 59.9));
        assert!(!rect.contains(40.0, 30.0));
        assert!(!rect.contains(30.0, 60.0));
        assert!(!rect.contains(9.9, 30.0));
    }

    #[test]
    fn button_click_emits_event_on_release_over_button() {
        let screen = Rect::screen(800.0, 600.0);
        let mut ui = Ui::new();
        ui.button(
            "play",
            UiRect::new(Anchor::Center, [-50.0, -20.0], [100.0, 40.0]),
            [0.2, 0.2, 0.2],
            [0.3, 0.3, 0.3],
            [0.4, 0.4, 0.4],
        );
        let mut input = InputState::new();
        move_to(&mut input, 400.0, 300.0);
        assert!(ui.handle_pointer(&input, &screen).is_empty());
        assert_eq!(ui.hovered(), Some("play"));
        press(&mut input, PRIMARY_MOUSE_BUTTON, true);
        assert!(ui.handle_pointer(&input, &screen).is_empty());
        press(&mut input, PRIMARY_MOUSE_BUTTON, false);
        let events = ui.handle_pointer(&input, &screen);
        assert_eq!(events, vec![UiEvent::Clicked("play".to_string())]);
    }

    #[test]
    fn button_release_outside_after_press_emits_nothing() {
        let screen = Rect::screen(800.0, 600.0);
        let mut ui = Ui::new();
        ui.button(
            "play",
            UiRect::new(Anchor::Center, [-50.0, -20.0], [100.0, 40.0]),
            [0.2; 3],
            [0.3; 3],
            [0.4; 3],
        );
        let mut input = InputState::new();
        move_to(&mut input, 400.0, 300.0);
        ui.handle_pointer(&input, &screen);
        press(&mut input, PRIMARY_MOUSE_BUTTON, true);
        ui.handle_pointer(&input, &screen);
        move_to(&mut input, 10.0, 10.0);
        press(&mut input, PRIMARY_MOUSE_BUTTON, false);
        assert!(ui.handle_pointer(&input, &screen).is_empty());
        assert_eq!(ui.hovered(), None);
    }

    #[test]
    fn press_outside_then_release_over_button_emits_nothing() {
        let screen = Rect::screen(800.0, 600.0);
        let mut ui = Ui::new();
        ui.button(
            "play",
            UiRect::new(Anchor::Center, [-50.0, -20.0], [100.0, 40.0]),
            [0.2; 3],
            [0.3; 3],
            [0.4; 3],
        );
        let mut input = InputState::new();
        move_to(&mut input, 10.0, 10.0);
        press(&mut input, PRIMARY_MOUSE_BUTTON, true);
        ui.handle_pointer(&input, &screen);
        move_to(&mut input, 400.0, 300.0);
        press(&mut input, PRIMARY_MOUSE_BUTTON, false);
        assert!(ui.handle_pointer(&input, &screen).is_empty());
    }

    #[test]
    fn topmost_button_wins_hit_test() {
        let screen = Rect::screen(800.0, 600.0);
        let mut ui = Ui::new();
        ui.button(
            "under",
            UiRect::new(Anchor::Center, [-50.0, -20.0], [100.0, 40.0]),
            [0.1; 3],
            [0.1; 3],
            [0.1; 3],
        );
        ui.button(
            "over",
            UiRect::new(Anchor::Center, [-30.0, -10.0], [60.0, 20.0]),
            [0.2; 3],
            [0.3; 3],
            [0.4; 3],
        );
        let mut input = InputState::new();
        move_to(&mut input, 400.0, 300.0);
        ui.handle_pointer(&input, &screen);
        assert_eq!(ui.hovered(), Some("over"));
    }

    #[test]
    fn draw_list_maps_unit_quad_onto_resolved_rects() {
        let screen = Rect::screen(800.0, 600.0);
        let mut ui = Ui::new();
        ui.panel(
            UiRect::new(Anchor::TopLeft, [10.0, 10.0], [200.0, 100.0]),
            [0.5, 0.0, 0.0],
        );
        ui.image(
            UiRect::new(Anchor::TopLeft, [20.0, 20.0], [40.0, 40.0]),
            [0.0, 0.5, 0.0],
        );
        let quads = ui.draw_list(&screen);
        assert_eq!(quads.len(), 2);
        assert_eq!(quads[0].color, [0.5, 0.0, 0.0]);
        assert_eq!(quads[0].mesh.positions.len(), 4);
        assert_eq!(quads[0].mesh.indices, vec![0, 1, 2, 0, 2, 3]);
        let corners = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let mapped: Vec<Vec3> = corners
            .iter()
            .map(|&c| quads[0].model.project_point(c))
            .collect();
        assert_eq!(
            mapped,
            vec![
                Vec3::new(10.0, 10.0, 0.0),
                Vec3::new(210.0, 10.0, 0.0),
                Vec3::new(210.0, 110.0, 0.0),
                Vec3::new(10.0, 110.0, 0.0),
            ]
        );
        assert_eq!(
            quads[1].model.project_point(Vec3::new(0.0, 0.0, 0.0)),
            Vec3::new(20.0, 20.0, 0.0)
        );
    }

    #[test]
    fn button_colors_follow_hover_and_pressed_state() {
        let screen = Rect::screen(800.0, 600.0);
        let mut ui = Ui::new();
        ui.button(
            "play",
            UiRect::new(Anchor::Center, [-50.0, -20.0], [100.0, 40.0]),
            [0.2; 3],
            [0.3; 3],
            [0.4; 3],
        );
        let mut input = InputState::new();
        let quads = ui.draw_list(&screen);
        assert_eq!(quads[0].color, [0.2; 3]);
        move_to(&mut input, 400.0, 300.0);
        ui.handle_pointer(&input, &screen);
        assert_eq!(ui.draw_list(&screen)[0].color, [0.3; 3]);
        press(&mut input, PRIMARY_MOUSE_BUTTON, true);
        ui.handle_pointer(&input, &screen);
        assert_eq!(ui.draw_list(&screen)[0].color, [0.4; 3]);
        press(&mut input, PRIMARY_MOUSE_BUTTON, false);
        ui.handle_pointer(&input, &screen);
        assert_eq!(ui.draw_list(&screen)[0].color, [0.3; 3]);
    }

    #[test]
    fn label_emits_text_overlay_item_at_resolved_rect() {
        let screen = Rect::screen(800.0, 600.0);
        let mut ui = Ui::new();
        ui.panel(
            UiRect::new(Anchor::TopLeft, [0.0, 0.0], [300.0, 100.0]),
            [0.1; 3],
        );
        ui.label(
            UiRect::new(Anchor::TopLeft, [10.0, 10.0], [200.0, 40.0]),
            "Score: 42",
            24.0,
            [1.0, 1.0, 1.0, 1.0],
        );
        let items = ui.text_overlay(&screen);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].text, "Score: 42");
        assert_eq!((items[0].x, items[0].y), (10.0, 10.0));
        assert_eq!(items[0].size_px, 24.0);
        // The label renders through the text overlay path, not as a quad.
        assert_eq!(ui.draw_list(&screen).len(), 1);
    }

    #[test]
    fn focus_defaults_to_first_button() {
        let ui = three_button_ui();
        assert_eq!(ui.focused(), Some("alpha"));
    }

    #[test]
    fn focus_order_wraps_forward_and_backward() {
        let mut ui = three_button_ui();
        let mut input = InputState::new();
        press_key(&mut input, KEY_RIGHT_ARROW);
        assert!(ui.handle_focus(&input).is_empty());
        assert_eq!(ui.focused(), Some("bravo"));
        release_key(&mut input, KEY_RIGHT_ARROW);
        press_key(&mut input, KEY_RIGHT_ARROW);
        ui.handle_focus(&input);
        assert_eq!(ui.focused(), Some("charlie"));
        release_key(&mut input, KEY_RIGHT_ARROW);
        press_key(&mut input, KEY_RIGHT_ARROW);
        ui.handle_focus(&input);
        assert_eq!(ui.focused(), Some("alpha"));
        release_key(&mut input, KEY_RIGHT_ARROW);
        press_key(&mut input, KEY_LEFT_ARROW);
        ui.handle_focus(&input);
        assert_eq!(ui.focused(), Some("charlie"));
        release_key(&mut input, KEY_LEFT_ARROW);
        press_key(&mut input, KEY_DOWN_ARROW);
        ui.handle_focus(&input);
        assert_eq!(ui.focused(), Some("alpha"));
        release_key(&mut input, KEY_DOWN_ARROW);
        press_key(&mut input, KEY_UP_ARROW);
        ui.handle_focus(&input);
        assert_eq!(ui.focused(), Some("charlie"));
    }

    #[test]
    fn named_axes_move_focus_on_direction_edges() {
        let mut ui = three_button_ui();
        let mut input = InputState::with_default_axes();
        let d = u32::from(b'd');
        let a = u32::from(b'a');
        press_key(&mut input, d);
        ui.handle_focus(&input);
        assert_eq!(ui.focused(), Some("bravo"));
        // Holding the key must not retrigger; only a direction change does.
        input.begin_frame(1.0 / 60.0);
        ui.handle_focus(&input);
        assert_eq!(ui.focused(), Some("bravo"));
        release_key(&mut input, d);
        press_key(&mut input, a);
        ui.handle_focus(&input);
        assert_eq!(ui.focused(), Some("alpha"));
    }

    #[test]
    fn submit_activates_the_focused_button() {
        let mut ui = three_button_ui();
        let mut input = InputState::new();
        press_key(&mut input, KEY_RETURN);
        let events = ui.handle_focus(&input);
        assert_eq!(events, vec![UiEvent::Clicked("alpha".to_string())]);
    }

    #[test]
    fn named_submit_and_cancel_buttons_drive_focus() {
        let mut ui = three_button_ui();
        ui.focus = Some("charlie".to_string());
        let mut input = InputState::new();
        input.set_axes(vec![
            button_axis("Submit", "space"),
            button_axis("Cancel", "escape"),
        ]);
        press_key(&mut input, KEY_SPACE);
        let events = ui.handle_focus(&input);
        assert_eq!(events, vec![UiEvent::Clicked("charlie".to_string())]);
        release_key(&mut input, KEY_SPACE);
        press_key(&mut input, KEY_ESCAPE);
        let events = ui.handle_focus(&input);
        assert_eq!(events, vec![UiEvent::Cancel]);
    }

    #[test]
    fn focus_survives_widget_rebuild() {
        let mut ui = three_button_ui();
        let mut input = InputState::new();
        press_key(&mut input, KEY_RIGHT_ARROW);
        ui.handle_focus(&input);
        assert_eq!(ui.focused(), Some("bravo"));
        // Immediate-mode rebuild: clear and re-declare the same buttons;
        // focus is tracked by button id, so it is preserved.
        ui.clear();
        for (id, x) in [("alpha", -120.0), ("bravo", 0.0), ("charlie", 120.0)] {
            ui.button(
                id,
                UiRect::new(Anchor::Center, [x, -20.0], [100.0, 40.0]),
                [0.2; 3],
                [0.3; 3],
                [0.4; 3],
            );
        }
        assert_eq!(ui.focused(), Some("bravo"));
    }

    #[test]
    fn focused_widget_emits_distinct_focus_indicator() {
        let screen = Rect::screen(800.0, 600.0);
        let button_rect = UiRect::new(Anchor::Center, [-50.0, -20.0], [100.0, 40.0]);
        let mut ui = Ui::new();
        ui.panel(
            UiRect::new(Anchor::TopLeft, [0.0, 0.0], [300.0, 200.0]),
            [0.2, 0.3, 0.4],
        );
        ui.label(
            UiRect::new(Anchor::TopLeft, [10.0, 10.0], [100.0, 20.0]),
            "hint",
            16.0,
            [1.0; 4],
        );
        let quads = ui.draw_list(&screen);
        assert_eq!(
            quads.len(),
            1,
            "panel only; no focusable widget, no indicator"
        );
        assert!(
            quads.iter().all(|quad| quad.color != FOCUS_INDICATOR_COLOR),
            "widgets must never use the indicator color"
        );
        ui.button("alpha", button_rect, [0.2; 3], [0.3; 3], [0.4; 3]);
        let quads = ui.draw_list(&screen);
        let indicators: Vec<&UiQuad> = quads
            .iter()
            .filter(|quad| quad.color == FOCUS_INDICATOR_COLOR)
            .collect();
        assert_eq!(
            indicators.len(),
            4,
            "the focused button draws a four-edge focus border"
        );
        for quad in &indicators {
            assert_eq!(quad.mesh.indices, vec![0, 1, 2, 0, 2, 3]);
        }
        assert_eq!(
            quads.len(),
            2 + 4,
            "panel + button quads plus the focus border"
        );
    }

    #[test]
    fn ui_camera_maps_screen_corners_to_ndc() {
        let camera = ui_camera(800.0, 600.0);
        let view_projection = camera.projection_matrix().mul_mat4(&ui_view(800.0, 600.0));
        let top_left = view_projection.project_point(Vec3::new(0.0, 0.0, 0.0));
        let bottom_right = view_projection.project_point(Vec3::new(800.0, 600.0, 0.0));
        assert!((top_left.x - -1.0).abs() < 1e-5);
        assert!((top_left.y - 1.0).abs() < 1e-5);
        assert!((bottom_right.x - 1.0).abs() < 1e-5);
        assert!((bottom_right.y - -1.0).abs() < 1e-5);
        let center = view_projection.project_point(Vec3::new(400.0, 300.0, 0.0));
        assert!(center.x.abs() < 1e-5);
        assert!(center.y.abs() < 1e-5);
    }
}

#[cfg(test)]
mod text_glyph_tests {
    use super::UiTextItem;
    use unitas_render::glyphs::{
        FontLibrary, GlyphAtlas, SwashCache, TextStyle, layout_text, shape_glyph_quads,
    };

    fn layout_at_scale(
        fonts: &mut FontLibrary,
        item: &UiTextItem,
        scale: f32,
    ) -> (f32, f32, usize) {
        let size_px = item.size_px * scale;
        let style = TextStyle::new(size_px, size_px * 1.25, item.color);
        let layout = layout_text(fonts, &item.text, None, &style);
        assert!(
            !layout.glyphs.is_empty(),
            "label text must shape into glyphs at scale {scale}"
        );
        let mut cache = SwashCache::new();
        let mut atlas = GlyphAtlas::new(1024, 1024);
        let quads = shape_glyph_quads(fonts, &mut cache, &mut atlas, &layout);
        assert!(
            !quads.is_empty(),
            "label text must pack into glyph quads at scale {scale}"
        );
        let max_quad_height = quads.iter().map(|quad| quad.height).fold(0.0f32, f32::max);
        (layout.width, max_quad_height, quads.len())
    }

    #[test]
    fn label_text_layout_produces_glyph_quads_at_multiple_resolutions() {
        let mut fonts = FontLibrary::new();
        if !fonts.load_system_fonts() {
            return;
        }
        let item = UiTextItem {
            text: "Score: 42".to_string(),
            x: 10.0,
            y: 10.0,
            size_px: 24.0,
            color: [1.0, 1.0, 1.0, 1.0],
        };
        let (width_lo, height_lo, quads_lo) = layout_at_scale(&mut fonts, &item, 1.0);
        let (width_hi, height_hi, quads_hi) = layout_at_scale(&mut fonts, &item, 2.0);
        assert!(width_lo > 0.0 && height_lo > 0.0);
        assert!(width_hi > width_lo, "higher resolution must reshape wider");
        assert!(
            (width_hi - width_lo * 2.0).abs() < width_lo * 0.2,
            "width should track the resolution scale, got {width_lo} vs {width_hi}"
        );
        assert!(
            height_hi > height_lo,
            "glyph quads must grow with the resolution scale"
        );
        assert!(quads_hi >= quads_lo);
    }
}
