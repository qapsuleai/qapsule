//! The pure helpers of the live path the expansion reads (the live path itself stays with
//! the authority: `qp-session` `data::live`).

use qp_schema::{Condition, Document, SeriesSource};

use qp_plane::values::find_series;

/// Every series `c` rests on (its series and, through every study input, theirs), as
/// [`LiveIndex::cond_info`] finds them, by lookup in `doc`.
pub fn closure_of(doc: &Document, c: &Condition) -> Vec<String> {
    let mut closure: Vec<String> = Vec::new();
    for id in cond_ids(c).into_iter().flatten() {
        let mut stack = vec![id];
        while let Some(cur) = stack.pop() {
            if closure.len() > 4096 || closure.iter().any(|x| x == cur) {
                continue;
            }
            closure.push(cur.to_owned());
            if let Some(SeriesSource::Study { inputs, .. }) =
                find_series(doc, cur).map(|s| &s.source)
            {
                stack.extend(inputs.iter().rev().map(String::as_str));
            }
        }
    }
    closure
}

/// The series ids a condition reads (at most two).
pub fn cond_ids(c: &Condition) -> [Option<&str>; 2] {
    match c {
        Condition::Threshold { series, .. } => [Some(series), None],
        Condition::Compare { left, right, .. } => [Some(left), Some(right)],
        Condition::Text { .. } => [None, None],
    }
}
