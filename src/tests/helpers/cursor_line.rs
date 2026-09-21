use super::*;
use crate::helpers::palette::Theme;

fn distance(left: Color, right: Color) -> u32 {
    let (Color::Rgb(lr, lg, lb), Color::Rgb(rr, rg, rb)) = (left, right) else {
        panic!("both colours must be rgb");
    };
    (lr.abs_diff(rr) as u32 + lg.abs_diff(rg) as u32 + lb.abs_diff(rb) as u32) / 3
}

#[test]
fn named_terminal_colours_cannot_be_mixed() {
    assert_eq!(cursor_line_background(Color::DarkGray, Color::Black, true), Color::DarkGray);
    assert_eq!(cursor_line_background(Color::DarkGray, Color::Black, false), Color::DarkGray);
}

#[test]
fn dark_cursor_lines_brighten_and_pale_ones_darken_when_focused() {
    assert_eq!(lift(66, 66, 66, 0.5), Color::Rgb(161, 161, 161));
    assert_eq!(lift(220, 224, 232, 0.5), Color::Rgb(110, 112, 116));
}

#[test]
fn losing_focus_settles_the_cursor_line_toward_the_zebra_stripe() {
    let resting = Color::Rgb(66, 66, 66);
    let zebra = Color::Rgb(33, 33, 33);
    let unfocused = cursor_line_background(resting, zebra, false);

    assert!(distance(unfocused, zebra) < distance(resting, zebra), "it must move toward the stripe");
    assert!(distance(unfocused, zebra) > 0, "but never onto it");
}

#[test]
fn every_theme_tells_its_two_states_apart_without_losing_the_zebra_stripe() {
    for preset in Theme::presets() {
        let theme = preset.theme;
        let resting = theme.cursor_line_color();
        let zebra = theme.zebra_color();
        if !matches!((resting, zebra), (Color::Rgb(..), Color::Rgb(..))) {
            continue;
        }

        let focused = cursor_line_background(resting, zebra, true);
        let unfocused = cursor_line_background(resting, zebra, false);
        let label = theme.label();

        assert!(distance(focused, unfocused) >= 20, "{label} only separates its two cursor states by {}", distance(focused, unfocused));
        // Dimming may narrow the gap to the stripe, but never more than the fall it was asked for.
        assert!(distance(unfocused, zebra) * 2 >= distance(resting, zebra), "{label} loses more than half its clearance over the zebra stripe when unfocused");
    }
}
