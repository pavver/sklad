//! Where the person is in a list of rows of one height, so the list can stay there when rows
//! above come or go under them.

use std::collections::HashMap;

use eframe::egui::Id;

/// The scroll offset of a list and the rows it showed, by key, top first.
#[derive(Clone, Debug, Default)]
pub struct Place {
    offset: f32,
    rows: Vec<(Id, usize)>,
}

impl Place {
    pub fn new(offset: f32, rows: Vec<(Id, usize)>) -> Self {
        Self { offset, rows }
    }

    /// The offset that keeps the first row seen that is still there where it was, now that the
    /// list holds `count` rows known by `key`, each `height` tall. `None` when nothing above it
    /// moved, or when none of the rows seen is left.
    pub fn offset_now(&self, count: usize, key: impl Fn(usize) -> Id, height: f32) -> Option<f32> {
        let &(first, row) = self.rows.first()?;
        if row < count && key(row) == first {
            return None;
        }
        let now: HashMap<Id, usize> = (0..count).map(|row| (key(row), row)).collect();
        self.rows.iter().find_map(|(seen, then)| {
            let moved = *now.get(seen)? as f32 - *then as f32;
            Some((self.offset + moved * height).max(0.0))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rows 5 to 7 of `keys`, seen 4 points into row 5 at rows 10 points tall.
    fn seen(keys: &[&str]) -> Place {
        Place::new(54.0, (5..8).map(|row| (Id::unique(keys[row]), row)).collect())
    }

    fn offset_in(place: &Place, keys: &[&str]) -> Option<f32> {
        place.offset_now(keys.len(), |row| Id::unique(keys[row]), 10.0)
    }

    const BEFORE: [&str; 10] = ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"];

    #[test]
    fn a_list_that_kept_its_rows_above_stays() {
        let place = seen(&BEFORE);
        assert_eq!(offset_in(&place, &BEFORE), None);
        assert_eq!(offset_in(&place, &["a", "b", "c", "d", "e", "f", "h", "j"]), None);
    }

    #[test]
    fn rows_that_leave_or_come_above_move_the_offset_with_them() {
        let place = seen(&BEFORE);
        assert_eq!(offset_in(&place, &["a", "d", "e", "f", "g", "h"]), Some(34.0));
        assert_eq!(
            offset_in(&place, &["new", "a", "b", "c", "d", "e", "f", "g"]),
            Some(64.0)
        );
    }

    #[test]
    fn when_the_top_row_leaves_the_next_one_seen_holds_the_place() {
        let place = seen(&BEFORE);
        assert_eq!(
            offset_in(&place, &["a", "b", "c", "d", "e", "g", "h"]),
            Some(44.0)
        );
        assert_eq!(offset_in(&place, &["x", "y"]), None, "nothing seen is left");
    }
}
