use super::*;
use crate::helpers::symbols::SymbolTheme;

// The art the splash draws, as the default symbol theme holds it.
fn wide() -> Vec<String> {
    SymbolTheme::default().splash.logo_wide
}

fn narrow() -> Vec<String> {
    SymbolTheme::default().splash.logo_narrow
}

// What a wordmark reads as at this moment, spans flattened back into the characters they carry.
fn drawn(rows: &[String], elapsed: Duration) -> Vec<String> {
    lines(rows, 5, &Theme::default(), elapsed).iter().map(|line| line.spans.iter().map(|span| span.content.as_ref()).collect()).collect()
}

// A spread of instants across more than one sweep, for the checks that hold whatever the sheen is
// doing.
fn moments() -> impl Iterator<Item = Duration> {
    (0..240).map(|step| Duration::from_millis(step * 47))
}

#[test]
fn at_rest_every_wordmark_is_the_art_as_it_was_drawn() {
    for rows in [wide(), narrow(), vec![SymbolTheme::default().splash.logo_compact]] {
        assert_eq!(drawn(&rows, Duration::ZERO), rows, "nothing has moved yet, so nothing should have changed");
    }
}

#[test]
fn every_cell_the_light_touches_rolls_through_the_ramp() {
    // The crest is cut into one band a tint, so a cell sees every one of them as the light passes.
    let art = wide();
    let sweep = sweep_over(&art);
    let seen: std::collections::BTreeSet<usize> = (0..6000).filter_map(|ms| sweep.band(4, 30, Duration::from_millis(ms))).collect();

    assert_eq!(seen.into_iter().collect::<Vec<_>>(), (0..PROFILE.len()).collect::<Vec<_>>(), "a cell should stand in every band of the crest as it goes by");
}

#[test]
fn at_rest_the_top_rows_are_grass_and_the_rest_are_green() {
    let theme = Theme::default();

    for (rows, bright) in [(wide(), 5), (narrow(), 4)] {
        assert_eq!(tint(0, 0, &rows, bright, &theme, Duration::ZERO), theme.COLOR_GRASS);
        assert_eq!(tint(bright, 0, &rows, bright, &theme, Duration::ZERO), theme.COLOR_GREEN, "the split is the one the splash has always drawn");
        assert_eq!(tint(rows.len() - 1, 0, &rows, bright, &theme, Duration::ZERO), theme.COLOR_GREEN);
    }
}

#[test]
fn the_grain_groups_cover_the_art_and_do_not_overlap() {
    let alphabet: String = GRAINS.concat();

    for glyph in wide().concat().chars().chain(narrow().concat().chars()).filter(|glyph| *glyph != ' ') {
        assert!(alphabet.contains(glyph), "{glyph:?} is drawn but belongs to no group, so it could never move");
    }
    for (index, group) in GRAINS.iter().enumerate() {
        for glyph in group.chars() {
            assert_eq!(GRAINS.iter().position(|other| other.contains(glyph)), Some(index), "{glyph:?} is in two groups");
        }
    }
}

#[test]
fn a_swapped_glyph_comes_from_its_own_group() {
    let art = wide();

    for elapsed in moments() {
        for (row, (was, now)) in art.iter().zip(drawn(&art, elapsed)).enumerate() {
            for (column, (before, after)) in was.chars().zip(now.chars()).enumerate() {
                if before == after {
                    continue;
                }
                let group = GRAINS.iter().find(|group| group.contains(before)).expect("a drawn glyph belongs to a group");
                assert!(group.contains(after), "{before:?} at {row},{column} became {after:?}, which is not its own weight");
            }
        }
    }
}

#[test]
fn a_blank_stays_blank() {
    let art = wide();

    for elapsed in moments() {
        for (was, now) in art.iter().zip(drawn(&art, elapsed)) {
            for after in was.chars().zip(now.chars()).filter(|(before, _)| *before == ' ').map(|(_, after)| after) {
                assert_eq!(after, ' ', "a hole in the letterform filled in at {elapsed:?}");
            }
        }
    }
}

#[test]
fn a_glyph_the_groups_do_not_name_is_left_alone() {
    // A symbol theme can replace the logo with anything at all, and glyphs the groups have no
    // weight for cannot be swapped without guessing at how heavy they are.
    let foreign = vec!["░▒▓█≈≋∷⊹".to_string(), "█▓▒░ ≋≋ ░▒▓█".to_string()];

    for elapsed in moments() {
        assert_eq!(drawn(&foreign, elapsed), foreign, "a themed logo of unknown glyphs moved at {elapsed:?}");
    }
}

#[test]
fn the_compact_logo_is_a_word_and_keeps_its_letters() {
    // `a` is one of the glyphs the art draws, so running the word through the grain would spell
    // something other than guitar -- which is what it did before `word` existed.
    let theme = Theme::default();

    for compact in [SymbolTheme::default().splash.logo_compact, SymbolTheme::ascii().splash.logo_compact] {
        for elapsed in moments() {
            let drawn: String = word(&compact, &theme, elapsed).spans.iter().map(|span| span.content.as_ref()).collect();
            assert_eq!(drawn, compact, "the word moved at {elapsed:?}");
        }
    }
}

#[test]
fn the_compact_logo_still_takes_the_sheen() {
    let theme = Theme::default();
    let compact = SymbolTheme::default().splash.logo_compact;
    let colours = |elapsed| word(&compact, &theme, elapsed).spans.iter().map(|span| span.style.fg).collect::<Vec<_>>();

    let at_rest = colours(Duration::ZERO);
    assert!(moments().any(|elapsed| colours(elapsed) != at_rest), "the word should still be lit by the sheen even though its letters hold");
}

#[test]
fn every_row_keeps_its_width_however_long_it_has_been_up() {
    for art in [wide(), narrow()] {
        let widths: Vec<usize> = art.iter().map(|row| row.chars().count()).collect();
        for elapsed in moments() {
            let now: Vec<usize> = drawn(&art, elapsed).iter().map(|row| row.chars().count()).collect();
            assert_eq!(now, widths, "the swap has to be one glyph for one glyph, at {elapsed:?}");
        }
    }
}

#[test]
fn the_grain_keeps_moving_and_stays_sparse() {
    let art = wide();
    let ink = art.concat().chars().filter(|glyph| *glyph != ' ').count();

    for elapsed in moments().skip(10) {
        let moved: usize = art.iter().zip(drawn(&art, elapsed)).map(|(was, now)| was.chars().zip(now.chars()).filter(|(before, after)| before != after).count()).sum();
        assert!(moved > 0, "the wordmark stood still at {elapsed:?}");
        assert!(moved * 4 < ink, "{moved} of {ink} cells moved at {elapsed:?}, which is noise rather than shimmer");
    }
}

#[test]
fn the_sheen_reaches_every_tint_over_one_sweep() {
    let theme = Theme::default();
    let art = wide();
    let mut seen: Vec<Color> = Vec::new();

    for elapsed in moments() {
        for (row, text) in art.iter().enumerate() {
            for column in 0..text.chars().count() {
                let colour = tint(row, column, &art, 5, &theme, elapsed);
                if !seen.contains(&colour) {
                    seen.push(colour);
                }
            }
        }
    }

    for colour in tints(&theme) {
        assert!(seen.contains(&colour), "the sheen never reached {colour:?}, so a tint is being paid for and not used");
    }
}

#[test]
fn the_sheen_is_quiet_at_both_ends_of_its_cycle() {
    // Nothing lit as the crest leaves and nothing lit as it comes back is what keeps the wrap from
    // showing as a jump.
    let art = wide();
    let sweep = sweep_over(&art);

    for step in [0, sweep.period - 1] {
        let elapsed = Duration::from_millis((step as u128 * SWEEP_MS / sweep.period as u128) as u64);
        for (row, text) in art.iter().enumerate() {
            assert!((0..text.chars().count()).all(|column| sweep.band(row, column, elapsed).is_none()), "step {step} of {} still had light on row {row}", sweep.period);
        }
    }
}

#[test]
fn the_crest_leans_across_the_rows() {
    // A band straight down the columns would read as a wipe; the lean is what makes it a sheen.
    let art = wide();
    let sweep = sweep_over(&art);
    let elapsed = Duration::from_millis(2000);
    let lit = |row: usize| (0..art[row].chars().count()).find(|column| sweep.band(row, *column, elapsed).is_some());

    let top = lit(2).expect("the crest should be on the wordmark by now");
    let bottom = lit(8).expect("and on every row of it at once");
    assert!(bottom + LEAN <= top, "the crest sat at {top} on row 2 and {bottom} on row 8, which is not a lean");
}

#[test]
fn a_sheen_is_sized_for_whatever_art_it_is_given() {
    // A symbol theme can make the logo any size, and the sweep has to cross it rather than sit on
    // one end of it.
    let tiny = sweep_over(&["##".to_string()]);
    let huge = sweep_over(&vec!["#".repeat(300); 30]);

    assert!(tiny.crest >= PROFILE.len(), "a crest narrower than the profile would skip steps as it rolled");
    assert!(huge.period > 300, "the crest has to have room to cross the art it is over");
}

#[test]
fn every_theme_gets_a_ramp_of_greens_that_are_all_different() {
    // A theme is free to map several roles onto one colour, and most do: nord publishes a single
    // green, everforest one colour for three of these slots. The ramp has to travel anyway.
    for preset in Theme::presets() {
        let ramp = tints(&preset.theme);
        if !ramp.iter().all(|tint| matches!(tint, Color::Rgb(..))) {
            continue;
        }
        for (index, tint) in ramp.iter().enumerate() {
            assert!(!ramp[..index].contains(tint), "{} repeats {tint:?} at stop {index} of its ramp: {ramp:?}", preset.label);
        }
    }
}

#[test]
fn a_theme_with_nothing_to_mix_keeps_the_colours_it_has() {
    // Named terminal colours have no channels to interpolate, which is also what keeps the
    // monochrome theme monochrome rather than inventing four greys for it.
    let named = [Color::Green, Color::Green, Color::LightGreen, Color::Cyan, Color::Cyan];

    assert_eq!(distinct(named), named);
}

#[test]
fn the_sheen_meets_the_resting_colour_at_both_ends() {
    // What the profile is for: a sheen that stopped on its darkest step would meet the rest of the
    // wordmark on a hard line instead of easing back into it.
    assert_eq!(PROFILE[0], 0, "the sheen has to start where the wordmark rests");
    assert_eq!(PROFILE[PROFILE.len() - 1], 0, "and come back to it rather than ending dark");

    let darkest = PROFILE.iter().position(|step| *step == *PROFILE.iter().max().expect("a profile")).expect("a darkest step");
    assert!(darkest < PROFILE.len() - 1, "there has to be something after the dark section to ease back through");
    assert!(PROFILE[darkest..].windows(2).all(|pair| pair[0] > pair[1]), "and it has to lighten the whole way out: {PROFILE:?}");
}

#[test]
fn the_sheen_lightens_before_it_darkens() {
    // Read left to right the light runs to its lightest first and its darkest second, which is the
    // direction the ramp itself is written in.
    let lightest = PROFILE.iter().position(|step| *step == *PROFILE.iter().min().expect("a profile")).expect("a lightest step");
    let darkest = PROFILE.iter().position(|step| *step == *PROFILE.iter().max().expect("a profile")).expect("a darkest step");

    assert!(lightest < darkest, "the light section belongs before the dark one, got {lightest} and {darkest}");
}

#[test]
fn the_drawn_logo_gives_way_to_the_smaller_one_at_the_breakpoint() {
    // The splash centres on this and a click on a recent repository is measured against it, so the
    // two have to see the same row count at every width.
    let splash = SymbolTheme::default().splash;

    assert_eq!(rows_at(WIDE_COLUMNS, &splash), splash.logo_wide.len());
    assert_eq!(rows_at(WIDE_COLUMNS - 1, &splash), splash.logo_narrow.len(), "a column short of the breakpoint is the smaller wordmark");
    assert_eq!(rows_at(NARROW_COLUMNS, &splash), splash.logo_narrow.len());
    assert_eq!(rows_at(NARROW_COLUMNS - 1, &splash), 1, "below that it is the word, on one row");
}

#[test]
fn every_wordmark_leaves_room_around_it_at_the_width_that_chooses_it() {
    let splash = SymbolTheme::default().splash;
    let widest = |art: &[String]| art.iter().map(|row| row.chars().count()).max().unwrap_or(0);

    assert!(widest(&splash.logo_wide) < WIDE_COLUMNS as usize, "the drawn logo has to fit the width that picks it, with room left over");
    assert!(widest(&splash.logo_narrow) < NARROW_COLUMNS as usize);
}
