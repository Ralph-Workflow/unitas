use unitas_render::{Camera, Mat4, Mesh, Vec3, look_at};

use crate::input::{InputState, KEY_MOUSE_0};

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
}

impl Widget {
    fn rect(&self) -> &UiRect {
        match self {
            Widget::Panel { rect, .. } => rect,
            Widget::Image { rect, .. } => rect,
            Widget::Button { rect, .. } => rect,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UiEvent {
    Clicked(String),
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
        }
    }

    pub fn clear(&mut self) {
        self.widgets.clear();
        self.hovered = None;
        self.active = None;
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

    pub fn hovered(&self) -> Option<&str> {
        self.hovered.and_then(|index| match &self.widgets[index] {
            Widget::Button { id, .. } => Some(id.as_str()),
            _ => None,
        })
    }

    pub fn handle_pointer(&mut self, input: &InputState, screen: &Rect) -> Vec<UiEvent> {
        let (px, py, _, _) = input.pointer();
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
        if input.is_mouse_button_pressed(KEY_MOUSE_0) {
            self.active = hovered;
        }
        if self.active.is_some() && self.active != hovered {
            self.active = None;
        }
        if input.is_mouse_button_released(KEY_MOUSE_0) {
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
        self.widgets
            .iter()
            .enumerate()
            .map(|(index, widget)| {
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
                };
                UiQuad {
                    mesh: unit_quad(),
                    model: quad_model(&rect),
                    color,
                }
            })
            .collect()
    }
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
    use crate::input::InputState;

    fn press(input: &mut InputState, button: u32, pressed: bool) {
        input.queue_mouse_button_event(button, pressed);
        input.begin_frame(1.0 / 60.0);
    }

    fn move_to(input: &mut InputState, x: f64, y: f64) {
        input.queue_pointer_move(x, y);
        input.begin_frame(1.0 / 60.0);
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
        press(&mut input, KEY_MOUSE_0, true);
        assert!(ui.handle_pointer(&input, &screen).is_empty());
        press(&mut input, KEY_MOUSE_0, false);
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
        press(&mut input, KEY_MOUSE_0, true);
        ui.handle_pointer(&input, &screen);
        move_to(&mut input, 10.0, 10.0);
        press(&mut input, KEY_MOUSE_0, false);
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
        press(&mut input, KEY_MOUSE_0, true);
        ui.handle_pointer(&input, &screen);
        move_to(&mut input, 400.0, 300.0);
        press(&mut input, KEY_MOUSE_0, false);
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
        press(&mut input, KEY_MOUSE_0, true);
        ui.handle_pointer(&input, &screen);
        assert_eq!(ui.draw_list(&screen)[0].color, [0.4; 3]);
        press(&mut input, KEY_MOUSE_0, false);
        ui.handle_pointer(&input, &screen);
        assert_eq!(ui.draw_list(&screen)[0].color, [0.3; 3]);
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
