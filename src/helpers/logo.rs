use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};
use std::time::Duration;

use crate::helpers::palette::{Theme, blend, distinct};

// Glyphs of like weight. A cell only ever swaps for another glyph from its own group, so the
// letterforms hold while the texture moves. Anything the groups do not name -- the letters of the
// compact word, whatever a custom symbol theme puts in the logo -- is left exactly as it is.
const GRAINS: [&str; 5] = ["#BG", "P564b", "?JYCa", "!71L", ".^:~"];

// How long a cell keeps a glyph before it may take another.
const HOLD_MS: u128 = 140;

// One cell in how many is carrying a borrowed glyph: away from the light, at the edges of the
// sheen, and through its middle. The light stirs the grain where it passes and leaves it alone
// behind.
const DENSITY: [u64; 3] = [18, 7, 4];

// How far the sheen leans: two columns a row, which is about forty-five degrees once the
// terminal's cells are twice as tall as they are wide.
const LEAN: usize = 2;

// How far along the ramp the light carries a cell, step by step across the crest and read left to
// right: out to the lightest tint, down through the ramp to the darkest, and back to where the
// wordmark rests. Easing to nothing at both ends is what keeps the sheen from meeting the resting
// colour on a hard line, and every step is measured along the same diagonal, so the tail leans
// with the rest of it.
const PROFILE: [isize; 13] = [0, -1, -2, -3, -2, -1, 0, 1, 2, 3, 2, 1, 0];

// How long one pass takes, crest and the quiet after it. The same at every size, so the small
// wordmark does not sweep faster than the big one.
const SWEEP_MS: u128 = 5200;

// A number that looks random for a cell but is the same every time it is asked for, so the
// animation needs no state of its own and no thread to drive it.
fn scatter(row: usize, column: usize, tick: u64) -> u64 {
    let mut seed = (row as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (column as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F) ^ tick.wrapping_mul(0x1656_67B1_9E37_79F9);
    seed ^= seed >> 30;
    seed = seed.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    seed ^= seed >> 27;
    seed = seed.wrapping_mul(0x94D0_49BB_1331_11EB);
    seed ^ (seed >> 31)
}

// Which of a cell's holds is current. Every cell carries its own offset into the beat, so the
// borrowed glyphs turn over one at a time instead of the whole wordmark blinking together.
fn hold(row: usize, column: usize, elapsed: Duration) -> u64 {
    let offset = scatter(row, column, 0) % HOLD_MS as u64;
    ((elapsed.as_millis() + offset as u128) / HOLD_MS) as u64
}

// What a cell draws now. Nothing has moved before the first beat, which is what makes the wordmark
// arrive as it was drawn and only then come alive.
fn grain(glyph: char, row: usize, column: usize, density: u64, elapsed: Duration) -> char {
    let Some(group) = GRAINS.iter().find(|group| group.contains(glyph)) else {
        return glyph;
    };
    let hold = hold(row, column, elapsed);
    if hold == 0 {
        return glyph;
    }
    let roll = scatter(row, column, hold);
    if !roll.is_multiple_of(density) {
        return glyph;
    }
    group.as_bytes()[(roll >> 8) as usize % group.len()] as char
}

// The band of light travelling across the wordmark. Its size comes off the art's own, so every
// logo gets the same picture at its own scale, and a symbol theme that replaces the art gets a
// sheen sized for whatever it put there.
struct Sweep {
    // The travel plus a quiet gap, so nothing is lit at either end of the cycle and the wrap back
    // to the start cannot show as a jump.
    period: usize,
    // How wide the crest is, measured along its travel.
    crest: usize,
}

impl Sweep {
    fn over(widest: usize, rows: usize) -> Self {
        let span = widest + LEAN * rows;
        let crest = (span / 4).max(PROFILE.len());
        Self { period: span * 4 / 3 + crest, crest }
    }

    // Where in the light this cell is standing: `None` clear of it, otherwise a band from 0 at the
    // crest's trailing edge to its last at the leading one. The crest is cut into one step per
    // entry in the profile, so a cell is carried along the whole of it as the light passes over.
    fn band(&self, row: usize, column: usize, elapsed: Duration) -> Option<usize> {
        let front = (elapsed.as_millis() * self.period as u128 / SWEEP_MS) as usize % self.period;
        // Zero is the crest still short of this cell, which is where every cycle starts, so a
        // wordmark at rest is the wordmark as it was drawn.
        let behind = front.saturating_sub(column + row * LEAN);
        if behind == 0 || behind > self.crest {
            return None;
        }
        // Counted back from the far end, so the ramp reads along the wordmark the way it is
        // written: the light leaves its first tint behind it and carries its last at the front.
        Some(PROFILE.len() - 1 - (behind - 1) * PROFILE.len() / self.crest)
    }
}

// How sparse the grain is where this cell stands. The light stirs it hardest through the middle of
// the crest and least out in the dark.
fn stir(band: Option<usize>) -> u64 {
    match band {
        None => DENSITY[0],
        Some(step) if PROFILE[step].abs() <= 1 => DENSITY[1],
        Some(_) => DENSITY[2],
    }
}

// Black, which the far end of the ramp is mixed toward to reach a green darker than any the
// palette names.
const SHADOW: Color = Color::Rgb(0, 0, 0);

// How far past the palette's own green the two darkest stops go.
const SHADES: [f32; 2] = [0.3, 0.6];

// The ramp the wordmark is lit from: yellow, down through the greens, and on into a green darker
// than the palette carries. The splash rests on the middle two -- the grass its top rows have
// always taken and the green below them -- and the light carries a cell out to either end as it
// rolls over it.
//
// The dark end is mixed rather than taken from `COLOR_LIGHT_GREEN_900`, which reads as the darkest
// green only on a dark theme: it is the colour behind an added line, so on the twelve light themes
// it is a pale wash and would have turned the ramp back up at its far end.
const TINTS: usize = 6;

fn tints(theme: &Theme) -> [Color; TINTS] {
    distinct([theme.COLOR_YELLOW, theme.COLOR_LIME, theme.COLOR_GRASS, theme.COLOR_GREEN, blend(theme.COLOR_GREEN, SHADOW, SHADES[0]), blend(theme.COLOR_GREEN, SHADOW, SHADES[1])])
}

// Where the wordmark rests: the grass for the top rows, the green for the rest.
const RESTING: usize = 2;

// What a row is lit in from the band it is standing in. `bright` is how many rows from the top take
// the lighter tone, which is the split the splash has always drawn; the light rides on that rather
// than replacing it, so a wordmark clear of the light is the one guitar has always shown.
fn tone(row: usize, band: Option<usize>, bright: usize, tints: &[Color; TINTS]) -> Color {
    let resting = (RESTING + usize::from(row >= bright)) as isize;
    let rolled = band.map_or(0, |step| PROFILE[step]);
    tints[(resting + rolled).clamp(0, TINTS as isize - 1) as usize]
}

// What a cell is lit in at this moment.
pub fn tint(row: usize, column: usize, rows: &[String], bright: usize, theme: &Theme, elapsed: Duration) -> Color {
    tone(row, sweep_over(rows).band(row, column, elapsed), bright, &tints(theme))
}

fn sweep_over(rows: &[String]) -> Sweep {
    Sweep::over(rows.iter().map(|row| row.chars().count()).max().unwrap_or(0), rows.len())
}

// The wordmark as it stands at `elapsed`: a line a row, grain and tint as the sheen leaves them. A
// tint depends only on where a cell is and never on what it draws, so a row comes back as a
// handful of spans rather than one a cell.
pub fn lines(rows: &[String], bright: usize, theme: &Theme, elapsed: Duration) -> Vec<Line<'static>> {
    let (sweep, tints) = (sweep_over(rows), tints(theme));
    rows.iter().enumerate().map(|(row, text)| line(row, text, bright, &sweep, &tints, elapsed, true)).collect()
}

// The compact logo, which is a word and not a picture. It takes the sheen, but its letters are
// left where they are: `a` is one of the glyphs the art draws, so scrambling this the way the art
// is scrambled would spell something other than guitar.
pub fn word(text: &str, theme: &Theme, elapsed: Duration) -> Line<'static> {
    line(0, text, 1, &Sweep::over(text.chars().count(), 1), &tints(theme), elapsed, false)
}

fn line(row: usize, text: &str, bright: usize, sweep: &Sweep, tints: &[Color; TINTS], elapsed: Duration, scrambled: bool) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut run = String::new();
    let mut colour: Option<Color> = None;

    for (column, glyph) in text.chars().enumerate() {
        let band = sweep.band(row, column, elapsed);
        let cell = tone(row, band, bright, tints);
        if let Some(previous) = colour
            && previous != cell
        {
            spans.push(Span::styled(std::mem::take(&mut run), Style::default().fg(previous)));
        }
        colour = Some(cell);
        run.push(if scrambled { grain(glyph, row, column, stir(band), elapsed) } else { glyph });
    }

    if let Some(colour) = colour {
        spans.push(Span::styled(run, Style::default().fg(colour)));
    }
    Line::from(spans)
}

#[cfg(test)]
#[path = "../tests/helpers/logo.rs"]
mod tests;
