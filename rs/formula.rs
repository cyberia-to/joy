//! formula — bracket-notation nox formulas ↔ the Reduction arena.
//!
//! Trident emits `.nox` output as bracket text, e.g. `[5 [[1 3] [1 5]]]`.
//! Atoms are decimal u64 values (reduced into Goldilocks); cells are
//! `[a b]`. N-ary brackets `[a b c]` fold right-nested: `[a [b c]]`.

use nebu::Goldilocks;
use nox::{Order, Reduction};

const ARENA_FULL: &str = "reduction arena full (formula too large)";

/// Parse bracket text into the arena. Returns the root `Order`.
pub fn parse<const N: usize>(r: &mut Reduction<N>, text: &str) -> Result<Order, String> {
    let bytes = text.as_bytes();
    let mut pos = 0usize;
    let id = parse_data(r, bytes, &mut pos)?;
    skip_ws(bytes, &mut pos);
    if pos != bytes.len() {
        return Err(format!("trailing input at byte {} of formula", pos));
    }
    Ok(id)
}

fn skip_ws(b: &[u8], pos: &mut usize) {
    while *pos < b.len() && b[*pos].is_ascii_whitespace() {
        *pos += 1;
    }
}

fn parse_data<const N: usize>(
    r: &mut Reduction<N>,
    b: &[u8],
    pos: &mut usize,
) -> Result<Order, String> {
    skip_ws(b, pos);
    if *pos >= b.len() {
        return Err("unexpected end of formula".to_string());
    }
    match b[*pos] {
        b'[' => {
            *pos += 1;
            let mut elems: Vec<Order> = Vec::new();
            loop {
                skip_ws(b, pos);
                if *pos >= b.len() {
                    return Err("unclosed '[' in formula".to_string());
                }
                if b[*pos] == b']' {
                    *pos += 1;
                    break;
                }
                elems.push(parse_data(r, b, pos)?);
            }
            if elems.len() < 2 {
                return Err(format!(
                    "cell needs at least 2 elements, got {}",
                    elems.len()
                ));
            }
            // Fold right: [a b c] = [a [b c]]
            let mut acc = elems[elems.len() - 1];
            for &e in elems[..elems.len() - 1].iter().rev() {
                acc = r.pair(e, acc).ok_or_else(|| ARENA_FULL.to_string())?;
            }
            Ok(acc)
        }
        b'0'..=b'9' => {
            let start = *pos;
            while *pos < b.len() && b[*pos].is_ascii_digit() {
                *pos += 1;
            }
            let s = core::str::from_utf8(&b[start..*pos])
                .map_err(|e| format!("invalid utf8 in atom: {}", e))?;
            let v: u64 = s
                .parse()
                .map_err(|e| format!("invalid atom '{}': {}", s, e))?;
            r.atom(Goldilocks::new(v))
                .ok_or_else(|| ARENA_FULL.to_string())
        }
        c => Err(format!(
            "unexpected character '{}' at byte {}",
            c as char, *pos
        )),
    }
}

/// Print data in bracket notation (inverse of `parse`).
pub fn print<const N: usize>(r: &Reduction<N>, id: Order) -> String {
    if let Some(v) = r.atom_value(id) {
        return v.as_u64().to_string();
    }
    match (r.head(id), r.tail(id)) {
        (Some(h), Some(t)) => format!("[{} {}]", print(r, h), print(r, t)),
        _ => "<invalid>".to_string(),
    }
}

/// Build the execution object (subject) from public inputs.
///
/// Trident's NoxCompiler binds parameters as a right-nested cons list
/// with the LAST parameter at the head: `[p_last [... [p_first 0]]]`
/// (`Scope::bind` conses each parameter onto the subject in order).
/// An empty input list yields the atom 0.
pub fn build_subject<const N: usize>(
    r: &mut Reduction<N>,
    values: &[u64],
) -> Result<Order, String> {
    let mut subject = r
        .atom(Goldilocks::new(0))
        .ok_or_else(|| ARENA_FULL.to_string())?;
    for v in values {
        let a = r
            .atom(Goldilocks::new(*v))
            .ok_or_else(|| ARENA_FULL.to_string())?;
        subject = r.pair(a, subject).ok_or_else(|| ARENA_FULL.to_string())?;
    }
    Ok(subject)
}

/// Flatten a result tree into its atom leaves, left-to-right.
pub fn leaves<const N: usize>(r: &Reduction<N>, id: Order) -> Result<Vec<u64>, String> {
    let mut out = Vec::new();
    let mut stack = vec![id];
    while let Some(n) = stack.pop() {
        if let Some(v) = r.atom_value(n) {
            out.push(v.as_u64());
        } else if let (Some(h), Some(t)) = (r.head(n), r.tail(n)) {
            stack.push(t);
            stack.push(h);
        } else {
            return Err("invalid data id in result".to_string());
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const N: usize = 1 << 12;

    #[test]
    fn atom_roundtrips() {
        let mut r = Reduction::<N>::new();
        let id = parse(&mut r, "42").unwrap();
        assert_eq!(print(&r, id), "42");
    }

    #[test]
    fn cell_roundtrips() {
        let mut r = Reduction::<N>::new();
        let text = "[5 [[1 3] [1 5]]]";
        let id = parse(&mut r, text).unwrap();
        assert_eq!(print(&r, id), text);
    }

    #[test]
    fn nary_cell_folds_right() {
        let mut r = Reduction::<N>::new();
        let id = parse(&mut r, "[4 [1 0] [1 1] [1 2]]").unwrap();
        assert_eq!(print(&r, id), "[4 [[1 0] [[1 1] [1 2]]]]");
    }

    #[test]
    fn whitespace_is_insignificant() {
        let mut r = Reduction::<N>::new();
        let a = parse(&mut r, "[5 [[1 3] [1 5]]]").unwrap();
        let b = parse(&mut r, "  [ 5\n [ [1   3] [1 5] ] ]\n").unwrap();
        // Hash-consing: identical trees share one Order.
        assert_eq!(a, b);
    }

    #[test]
    fn unclosed_bracket_errors() {
        let mut r = Reduction::<N>::new();
        assert!(parse(&mut r, "[5 [1 3]").is_err());
    }

    #[test]
    fn trailing_input_errors() {
        let mut r = Reduction::<N>::new();
        assert!(parse(&mut r, "[1 2] 7").is_err());
    }

    #[test]
    fn singleton_cell_errors() {
        let mut r = Reduction::<N>::new();
        assert!(parse(&mut r, "[5]").is_err());
    }

    #[test]
    fn subject_puts_last_input_at_head() {
        let mut r = Reduction::<N>::new();
        let s = build_subject(&mut r, &[3, 5]).unwrap();
        // [5 [3 0]] — last parameter at the head (axis 2).
        assert_eq!(print(&r, s), "[5 [3 0]]");
    }

    #[test]
    fn empty_subject_is_zero() {
        let mut r = Reduction::<N>::new();
        let s = build_subject(&mut r, &[]).unwrap();
        assert_eq!(print(&r, s), "0");
    }

    #[test]
    fn leaves_flatten_left_to_right() {
        let mut r = Reduction::<N>::new();
        let id = parse(&mut r, "[[1 2] [3 4]]").unwrap();
        assert_eq!(leaves(&r, id).unwrap(), vec![1, 2, 3, 4]);
    }
}
