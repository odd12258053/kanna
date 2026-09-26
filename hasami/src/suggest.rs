//! "Did you mean" suggestions (feature `suggest`): a self-contained
//! Jaro-Winkler similarity, no dependencies.

/// The candidate most similar to `input`, if it is similar enough to be a
/// plausible typo.
pub(crate) fn closest<'a>(
    input: &str,
    candidates: impl IntoIterator<Item = &'a str>,
) -> Option<&'a str> {
    let mut best: Option<(&str, f64)> = None;
    for c in candidates {
        let score = jaro_winkler(input, c);
        if score >= THRESHOLD && best.is_none_or(|(_, s)| score > s) {
            best = Some((c, score));
        }
    }
    best.map(|(c, _)| c)
}

const THRESHOLD: f64 = 0.8;

/// Jaro-Winkler similarity in `0.0..=1.0`, on Unicode scalar values.
pub(crate) fn jaro_winkler(a: &str, b: &str) -> f64 {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let jaro = jaro(&a, &b);
    if jaro <= 0.7 {
        return jaro;
    }
    let prefix = a.iter().zip(&b).take(4).take_while(|(x, y)| x == y).count();
    jaro + 0.1 * prefix as f64 * (1.0 - jaro)
}

fn jaro(a: &[char], b: &[char]) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let window = (a.len().max(b.len()) / 2).saturating_sub(1);
    let mut a_hit = vec![false; a.len()];
    let mut b_hit = vec![false; b.len()];
    let mut matches = 0usize;
    for (i, &ca) in a.iter().enumerate() {
        let lo = i.saturating_sub(window);
        let hi = (i + window + 1).min(b.len());
        for j in lo..hi {
            if !b_hit[j] && b[j] == ca {
                a_hit[i] = true;
                b_hit[j] = true;
                matches += 1;
                break;
            }
        }
    }
    if matches == 0 {
        return 0.0;
    }
    let mut transpositions = 0usize;
    let mut j = 0usize;
    for (i, &ca) in a.iter().enumerate() {
        if !a_hit[i] {
            continue;
        }
        while !b_hit[j] {
            j += 1;
        }
        if ca != b[j] {
            transpositions += 1;
        }
        j += 1;
    }
    let m = matches as f64;
    (m / a.len() as f64 + m / b.len() as f64 + (m - transpositions as f64 / 2.0) / m) / 3.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn similarity_is_sane() {
        assert!((jaro_winkler("martha", "marhta") - 0.961).abs() < 0.01);
        assert!((jaro_winkler("dixon", "dicksonx") - 0.813).abs() < 0.01);
        assert_eq!(jaro_winkler("", ""), 1.0);
        assert_eq!(jaro_winkler("a", ""), 0.0);
        assert_eq!(jaro_winkler("abc", "abc"), 1.0);
    }

    #[test]
    fn closest_picks_typos_but_not_unrelated_names() {
        let c = ["--number", "--shout", "--help"];
        assert_eq!(closest("--nubmer", c), Some("--number"));
        assert_eq!(closest("--numbre", c), Some("--number"));
        assert_eq!(closest("--shuot", c), Some("--shout"));
        assert_eq!(closest("--zzzzzz", c), None);
        assert_eq!(closest("-n", c), None);
    }
}
