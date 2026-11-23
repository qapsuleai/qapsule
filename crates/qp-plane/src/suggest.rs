//! `did_you_mean` suggestions (`docs/PROTOCOL.md` §13): at most 5 names whose edit
//! distance to the given one is at most 3, or that contain it (or are contained in it);
//! sorted by distance, then alphabetically.

/// Levenshtein distance over Unicode scalar values.
#[must_use]
pub fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let sub = prev[j] + usize::from(ca != cb);
            cur[j + 1] = sub.min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

/// Up to five candidates close to `given` (exact matches excluded, duplicates removed).
#[must_use]
pub fn did_you_mean<'a, I>(given: &str, candidates: I) -> Vec<String>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut scored: Vec<(usize, &str)> = Vec::new();
    for c in candidates {
        if c == given || scored.iter().any(|(_, s)| *s == c) {
            continue;
        }
        let d = edit_distance(given, c);
        let substring = !given.is_empty() && (c.contains(given) || given.contains(c));
        if d <= 3 || substring {
            scored.push((d, c));
        }
    }
    scored.sort_unstable();
    scored
        .into_iter()
        .take(5)
        .map(|(_, s)| s.to_owned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance() {
        assert_eq!(edit_distance("kitten", "sitting"), 3);
        assert_eq!(edit_distance("", "abc"), 3);
        assert_eq!(edit_distance("rsi", "rsi14"), 2);
    }

    #[test]
    fn suggestions_sorted_and_capped() {
        let c = ["rsi14", "rsi7", "macd", "bb20", "rsi", "rsi_long_name"];
        assert_eq!(
            did_you_mean("rsi", c.iter().copied()),
            vec!["rsi7", "rsi14", "rsi_long_name"]
        );
        assert_eq!(
            did_you_mean("parms", ["params", "id"].iter().copied()),
            vec!["params"]
        );
        let many = ["a1", "a2", "a3", "a4", "a5", "a6"];
        assert_eq!(did_you_mean("a", many.iter().copied()).len(), 5);
    }
}
