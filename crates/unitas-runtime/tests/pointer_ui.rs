use unitas_runtime::input::{
    InputState, KEY_LEFT_ARROW, KEY_RETURN, KEY_RIGHT_ARROW, PointerFrame,
};
use unitas_runtime::ui::{Anchor, Rect, Ui, UiEvent, UiRect};

const FRAME_SECONDS: f64 = 1.0 / 60.0;
const SCREEN_WIDTH: f32 = 800.0;
const SCREEN_HEIGHT: f32 = 600.0;
const BUTTON_CENTER: (f32, f32) = (400.0, 300.0);
const OUTSIDE_BUTTON: (f32, f32) = (10.0, 10.0);

fn screen() -> Rect {
    Rect::screen(SCREEN_WIDTH, SCREEN_HEIGHT)
}

fn one_button_ui(id: &str) -> Ui {
    let mut ui = Ui::new();
    ui.button(
        id,
        UiRect::new(Anchor::Center, [-50.0, -20.0], [100.0, 40.0]),
        [0.2; 3],
        [0.3; 3],
        [0.4; 3],
    );
    ui
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
fn pointer_click_activates_a_button_headless() {
    let screen = screen();
    let mut ui = one_button_ui("play");
    let mut input = InputState::new();

    input.inject_pointer_frame(BUTTON_CENTER, false);
    input.begin_frame(FRAME_SECONDS);
    assert!(ui.handle_pointer(&input, &screen).is_empty());
    assert_eq!(ui.hovered(), Some("play"));

    input.inject_pointer_frame(BUTTON_CENTER, true);
    input.begin_frame(FRAME_SECONDS);
    assert!(ui.handle_pointer(&input, &screen).is_empty());

    input.inject_pointer_frame(BUTTON_CENTER, false);
    input.begin_frame(FRAME_SECONDS);
    let events = ui.handle_pointer(&input, &screen);
    assert_eq!(events, vec![UiEvent::Clicked("play".to_string())]);
}

#[test]
fn pointer_outside_the_button_rect_clicks_nothing() {
    let screen = screen();
    let mut ui = one_button_ui("play");
    let mut input = InputState::new();

    input.inject_pointer_frame(OUTSIDE_BUTTON, false);
    input.begin_frame(FRAME_SECONDS);
    assert!(ui.handle_pointer(&input, &screen).is_empty());
    assert_eq!(ui.hovered(), None);

    input.inject_pointer_frame(OUTSIDE_BUTTON, true);
    input.begin_frame(FRAME_SECONDS);
    assert!(ui.handle_pointer(&input, &screen).is_empty());

    input.inject_pointer_frame(OUTSIDE_BUTTON, false);
    input.begin_frame(FRAME_SECONDS);
    assert!(ui.handle_pointer(&input, &screen).is_empty());
    assert_eq!(ui.hovered(), None);
}

#[test]
fn keyboard_focus_still_cycles_and_enter_submits() {
    let mut ui = three_button_ui();
    let mut input = InputState::new();

    input.inject_key(KEY_RIGHT_ARROW, true);
    input.begin_frame(FRAME_SECONDS);
    assert!(ui.handle_focus(&input).is_empty());
    assert_eq!(ui.focused(), Some("bravo"));

    input.inject_key(KEY_LEFT_ARROW, true);
    input.begin_frame(FRAME_SECONDS);
    assert!(ui.handle_focus(&input).is_empty());
    assert_eq!(ui.focused(), Some("alpha"));

    input.inject_key(KEY_RETURN, true);
    input.begin_frame(FRAME_SECONDS);
    let events = ui.handle_focus(&input);
    assert_eq!(events, vec![UiEvent::Clicked("alpha".to_string())]);
}

#[test]
fn pointer_frame_reports_position_and_primary_transitions() {
    let mut input = InputState::new();
    assert_eq!(
        input.pointer_frame(),
        PointerFrame {
            position: (0.0, 0.0),
            primary_held: false,
            primary_pressed: false,
            primary_released: false,
        }
    );

    input.inject_pointer_frame((12.5, 40.0), false);
    input.begin_frame(FRAME_SECONDS);
    assert_eq!(
        input.pointer_frame(),
        PointerFrame {
            position: (12.5, 40.0),
            primary_held: false,
            primary_pressed: false,
            primary_released: false,
        }
    );

    input.inject_pointer_frame((12.5, 40.0), true);
    input.begin_frame(FRAME_SECONDS);
    assert_eq!(
        input.pointer_frame(),
        PointerFrame {
            position: (12.5, 40.0),
            primary_held: true,
            primary_pressed: true,
            primary_released: false,
        }
    );

    input.inject_pointer_frame((12.5, 40.0), true);
    input.begin_frame(FRAME_SECONDS);
    assert_eq!(
        input.pointer_frame(),
        PointerFrame {
            position: (12.5, 40.0),
            primary_held: true,
            primary_pressed: false,
            primary_released: false,
        }
    );

    input.inject_pointer_frame((12.5, 40.0), false);
    input.begin_frame(FRAME_SECONDS);
    assert_eq!(
        input.pointer_frame(),
        PointerFrame {
            position: (12.5, 40.0),
            primary_held: false,
            primary_pressed: false,
            primary_released: true,
        }
    );
}
