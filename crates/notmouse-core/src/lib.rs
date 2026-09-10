//! Platform-neutral interaction primitives for `!mouse`.

/// Keys used for generated hints, ordered roughly by typing comfort.
///
/// Keeping this alphabet free of modifiers and punctuation makes hints work on
/// common keyboard layouts and avoids conflicts with mode-control keys.
pub const DEFAULT_HINT_ALPHABET: &str = "asdfjklghqwertyuiopzxcvbnm";

/// 9 home-row keys used for the 3x3 macro and micro matrix layout.
pub const HOME_ROW_ALPHABET: &str = "asdfjklgh";

/// A rectangle expressed in logical screen coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    /// Creates a new rectangle in logical coordinates.
    #[must_use]
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Computes the center coordinates `(x, y)` of the rectangle.
    #[must_use]
    pub fn center(&self) -> (f64, f64) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }
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

/// A 2-stroke spatial matrix for 2-key screen navigation.
///
/// Divides a bounding rectangle into a macro grid of primary zones,
/// each of which contains an identical micro grid of secondary zones.
/// This allows pinpointing any region on screen in exactly two keystrokes.
#[derive(Clone, Debug, PartialEq)]
pub struct SpatialMatrix2D {
    pub bounds: Rect,
    pub alphabet: String,
    pub rows: u32,
    pub columns: u32,
}

impl SpatialMatrix2D {
    /// Creates a default 3x3 2-stroke matrix using [`HOME_ROW_ALPHABET`].
    #[must_use]
    pub fn new(bounds: Rect) -> Self {
        Self {
            bounds,
            alphabet: HOME_ROW_ALPHABET.to_owned(),
            rows: 3,
            columns: 3,
        }
    }

    /// Creates a 2-stroke matrix with custom bounds, alphabet, and dimensions.
    ///
    /// # Panics
    ///
    /// Panics if `rows` or `columns` is zero, or if `alphabet.chars().count()`
    /// does not match `rows * columns`.
    #[must_use]
    pub fn with_alphabet(bounds: Rect, alphabet: &str, rows: u32, columns: u32) -> Self {
        assert!(rows > 0, "rows must be greater than zero");
        assert!(columns > 0, "columns must be greater than zero");
        let count = rows
            .checked_mul(columns)
            .expect("dimensions exceed capacity");
        let expected_len = usize::try_from(count).expect("dimensions do not fit this platform");
        assert_eq!(
            alphabet.chars().count(),
            expected_len,
            "alphabet length must equal rows * columns"
        );
        Self {
            bounds,
            alphabet: alphabet.to_owned(),
            rows,
            columns,
        }
    }

    /// Returns the primary macro zones covering the entire bounding rectangle.
    ///
    /// # Panics
    ///
    /// Panics if the grid dimensions exceed target platform memory capacity.
    #[must_use]
    pub fn macro_zones(&self) -> Vec<Zone> {
        let count = self
            .rows
            .checked_mul(self.columns)
            .expect("dimensions exceed capacity");
        let symbols: Vec<char> = self.alphabet.chars().collect();
        let zone_width = self.bounds.width / f64::from(self.columns);
        let zone_height = self.bounds.height / f64::from(self.rows);

        (0..count)
            .map(|index| {
                let row = index / self.columns;
                let col = index % self.columns;
                let hint_idx = usize::try_from(index).expect("index fits platform");
                let hint = symbols[hint_idx].to_string();
                let bounds = Rect {
                    x: self.bounds.x + f64::from(col) * zone_width,
                    y: self.bounds.y + f64::from(row) * zone_height,
                    width: zone_width,
                    height: zone_height,
                };
                Zone { hint, bounds }
            })
            .collect()
    }

    /// Returns the micro sub-zones inside the macro zone identified by `macro_hint`.
    #[must_use]
    pub fn micro_zones(&self, macro_hint: char) -> Option<Vec<Zone>> {
        let macro_zone = self
            .macro_zones()
            .into_iter()
            .find(|zone| zone.hint.starts_with(macro_hint))?;
        let sub_matrix =
            Self::with_alphabet(macro_zone.bounds, &self.alphabet, self.rows, self.columns);
        let micro = sub_matrix
            .macro_zones()
            .into_iter()
            .map(|sub| Zone {
                hint: format!("{macro_hint}{}", sub.hint),
                bounds: sub.bounds,
            })
            .collect();
        Some(micro)
    }

    /// Resolves a 2-stroke sequence (e.g. `'d'`, `'k'`) to the final target zone.
    #[must_use]
    pub fn resolve(&self, stroke1: char, stroke2: char) -> Option<Zone> {
        let micro = self.micro_zones(stroke1)?;
        let expected_hint = format!("{stroke1}{stroke2}");
        micro.into_iter().find(|zone| zone.hint == expected_hint)
    }

    /// Returns all resolved 2-stroke target zones across the entire screen.
    ///
    /// # Panics
    ///
    /// Panics if internal macro hints are malformed or empty.
    #[must_use]
    pub fn all_zones(&self) -> Vec<Zone> {
        let mut zones = Vec::new();
        for macro_zone in self.macro_zones() {
            let macro_char = macro_zone
                .hint
                .chars()
                .next()
                .expect("macro hint should not be empty");
            if let Some(micro) = self.micro_zones(macro_char) {
                zones.extend(micro);
            }
        }
        zones
    }
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

    #[test]
    fn rect_center_computes_midpoint() {
        let rect = Rect::new(100.0, 200.0, 50.0, 80.0);
        assert_eq!(rect.center(), (125.0, 240.0));
    }

    #[test]
    fn spatial_matrix_produces_nine_macro_zones() {
        let matrix = SpatialMatrix2D::new(Rect::new(0.0, 0.0, 900.0, 900.0));
        let macro_zones = matrix.macro_zones();
        assert_eq!(macro_zones.len(), 9);
        assert_eq!(macro_zones[0].hint, "a");
        assert_eq!(macro_zones[8].hint, "h");
        assert_eq!(macro_zones[0].bounds, Rect::new(0.0, 0.0, 300.0, 300.0));
    }

    #[test]
    fn spatial_matrix_micro_zones_nested_correctly() {
        let matrix = SpatialMatrix2D::new(Rect::new(0.0, 0.0, 900.0, 900.0));
        let micro = matrix.micro_zones('a').expect("micro zones for 'a'");
        assert_eq!(micro.len(), 9);
        assert_eq!(micro[0].hint, "aa");
        assert_eq!(micro[1].hint, "as");
        assert_eq!(micro[0].bounds, Rect::new(0.0, 0.0, 100.0, 100.0));
    }

    #[test]
    fn spatial_matrix_resolves_two_strokes() {
        let matrix = SpatialMatrix2D::new(Rect::new(0.0, 0.0, 900.0, 900.0));
        let resolved = matrix.resolve('a', 's').expect("resolve 'as'");
        assert_eq!(resolved.hint, "as");
        assert_eq!(resolved.bounds, Rect::new(100.0, 0.0, 100.0, 100.0));
        assert_eq!(resolved.bounds.center(), (150.0, 50.0));
    }

    #[test]
    fn spatial_matrix_all_zones_has_81_cells() {
        let matrix = SpatialMatrix2D::new(Rect::new(0.0, 0.0, 900.0, 900.0));
        let all = matrix.all_zones();
        assert_eq!(all.len(), 81);
        assert_eq!(all[0].hint, "aa");
        assert_eq!(all[80].hint, "hh");
    }
}
