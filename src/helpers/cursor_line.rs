use ratatui::style::Color;

// How far the focused cursor line is lifted clear of its resting colour, as a share of the room
// left between it and whichever end of the range it is heading for.
const FOCUSED_LIFT: f32 = 0.13;

// How far the unfocused cursor line settles back toward the zebra stripe, the quietest row colour
// the list already paints. Going by a share of that gap rather than a fixed step is what keeps
// tight palettes from settling onto the stripe and vanishing.
const UNFOCUSED_FALL: f32 = 0.35;

// Cursor lines darker than this brighten when focused; paler ones darken instead, because that is
// where the room to move is.
const MIDPOINT: f32 = 128.0;

// The cursor line's background: lifted clear while the terminal has focus, settled back toward the
// zebra stripe when it does not. Themes built from named terminal colours have no channels to mix,
// so they keep the resting colour either way.
pub fn cursor_line_background(resting: Color, zebra: Color, is_focused: bool) -> Color {
    let Color::Rgb(red, green, blue) = resting else {
        return resting;
    };

    if is_focused {
        return lift(red, green, blue, FOCUSED_LIFT);
    }

    let Color::Rgb(zebra_red, zebra_green, zebra_blue) = zebra else {
        return resting;
    };

    Color::Rgb(toward(red, zebra_red, UNFOCUSED_FALL), toward(green, zebra_green, UNFOCUSED_FALL), toward(blue, zebra_blue, UNFOCUSED_FALL))
}

fn lift(red: u8, green: u8, blue: u8, amount: f32) -> Color {
    let luminance = 0.299 * f32::from(red) + 0.587 * f32::from(green) + 0.114 * f32::from(blue);
    let is_brightening = luminance < MIDPOINT;

    let lifted = |channel: u8| {
        let channel = f32::from(channel);
        let moved = if is_brightening { channel + (255.0 - channel) * amount } else { channel - channel * amount };
        moved.round().clamp(0.0, 255.0) as u8
    };

    Color::Rgb(lifted(red), lifted(green), lifted(blue))
}

fn toward(channel: u8, target: u8, amount: f32) -> u8 {
    let channel = f32::from(channel);
    (channel + (f32::from(target) - channel) * amount).round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
#[path = "../tests/helpers/cursor_line.rs"]
mod tests;
