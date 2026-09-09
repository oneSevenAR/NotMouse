//! Platform-neutral interaction primitives for `!mouse`.

/// Keys used for generated hints, ordered roughly by typing comfort.
///
/// Keeping this alphabet free of modifiers and punctuation makes hints work on
/// common keyboard layouts and avoids conflicts with mode-control keys.
pub const DEFAULT_HINT_ALPHABET: &str = "asdfjklghqwertyuiopzxcvbnm";

/// A rectangle expressed in logical screen coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// A selectable part of the screen.
#[derive(Clone, Debug, PartialEq)]
pub struct Zone {
    pub hint: String,
    pub bounds: Rect,
}

/// Splits `bounds` into an evenly sized grid and assigns each zone a hint.
///
/// Zones are ordered from left to right, then top to bottom.
///
/// # Panics
///
/// Panics when either grid dimension is zero.
#[must_use]
pub fn grid(bounds: Rect, rows: u32, columns: u32) -> Vec<Zone> {
    assert!(rows > 0, "a grid needs at least one row");
    assert!(columns > 0, "a grid needs at least one column");

    let count = rows
        .checked_mul(columns)
        .expect("the requested grid is too large");
    let hint_count = usize::try_from(count).expect("the grid does not fit this platform");
    let hints = generate_hints(hint_count, DEFAULT_HINT_ALPHABET);
    let zone_width = bounds.width / f64::from(columns);
    let zone_height = bounds.height / f64::from(rows);

    hints
        .into_iter()
        .zip(0..count)
        .map(|(hint, index)| {
            let row = index / columns;
            let column = index % columns;

            Zone {
                hint,
                bounds: Rect {
                    x: bounds.x + f64::from(column) * zone_width,
                    y: bounds.y + f64::from(row) * zone_height,
                    width: zone_width,
                    height: zone_height,
                },
            }
        })
        .collect()
}

/// Generates the shortest fixed-width hints that can represent `count` items.
///
/// Fixed-width hints are deliberately used in the first prototype: no valid
/// hint can be a prefix of another, so a selection is never ambiguous.
///
/// # Panics
///
/// Panics when the alphabet contains fewer than two distinct characters.
#[must_use]
pub fn generate_hints(count: usize, alphabet: &str) -> Vec<String> {
    if count == 0 {
        return Vec::new();
    }

    let symbols: Vec<char> = alphabet.chars().collect();
    assert!(
        symbols.len() >= 2,
        "the hint alphabet needs at least two keys"
    );

    let mut distinct = symbols.clone();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(
        distinct.len(),
        symbols.len(),
        "the hint alphabet cannot contain duplicate keys"
    );

    let base = symbols.len();
    let mut width = 1;
    let mut capacity = base;
    while capacity < count {
        capacity = capacity
            .checked_mul(base)
            .expect("the requested hint set is too large");
        width += 1;
    }

    (0..count)
        .map(|mut value| {
            let mut hint = vec![symbols[0]; width];
            for position in (0..width).rev() {
                hint[position] = symbols[value % base];
                value /= base;
            }
            hint.into_iter().collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hints_start_with_home_row_keys() {
        assert_eq!(
            generate_hints(5, DEFAULT_HINT_ALPHABET),
            ["a", "s", "d", "f", "j"]
        );
    }

    #[test]
    fn hints_expand_to_an_unambiguous_width() {
        assert_eq!(generate_hints(5, "as"), ["aaa", "aas", "asa", "ass", "saa"]);
    }

    #[test]
    fn empty_target_set_has_no_hints() {
        assert!(generate_hints(0, DEFAULT_HINT_ALPHABET).is_empty());
    }

    #[test]
    fn grid_covers_the_requested_bounds() {
        let zones = grid(
            Rect {
                x: 10.0,
                y: 20.0,
                width: 300.0,
                height: 180.0,
            },
            2,
            3,
        );

        assert_eq!(zones.len(), 6);
        assert_eq!(
            zones[0].bounds,
            Rect {
                x: 10.0,
                y: 20.0,
                width: 100.0,
                height: 90.0,
            }
        );
        assert!((zones[5].bounds.x - 210.0).abs() < f64::EPSILON);
        assert!((zones[5].bounds.y - 110.0).abs() < f64::EPSILON);
    }
}
