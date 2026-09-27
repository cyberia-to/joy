//! formula — bracket-notation nox formulas ↔ the Reduction arena.
//!
//! Trident emits `.nox` output as bracket text, e.g. `[5 [[1 3] [1 5]]]`.
//! Atoms are decimal u64 values (reduced into Goldilocks); cells are
//! `[a b]`. N-ary brackets `[a b c]` fold right-nested: `[a [b c]]`.

use nebu::Goldilocks;
use nox::{Order, Reduction};

const ARENA_FULL: &str = "reduction arena full (formula too large)";
/// Raw text admission is independent of the VM reduction budget.
pub const MAX_FORMULA_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_FORMULA_DEPTH: usize = 4096;
pub const MAX_FORMULA_NODES: usize = 1 << 20;
/// Flat CLI/runtime output is at most 8 MiB of canonical field words.
pub const MAX_OUTPUT_WORDS: usize = 1 << 20;

/// Parse bracket text into the arena. Returns the root `Order`.
pub fn parse<const N: usize>(r: &mut Reduction<N>, text: &str) -> Result<Order, String> {
    if text.len() > MAX_FORMULA_BYTES {
        return Err("formula byte limit".into());
    }
    let bytes = text.as_bytes();
    let mut pos = 0usize;
    let mut nodes = 0usize;
    let mut frames: Vec<Vec<(Order, usize)>> = vec![Vec::new()];
    while pos < bytes.len() {
        match bytes[pos] {
            c if c.is_ascii_whitespace() => pos += 1,
            b'[' => {
                if frames.len() > MAX_FORMULA_DEPTH {
                    return Err("formula depth limit".into());
                }
                frames.push(Vec::new());
                pos += 1;
            }
            b']' => {
                if frames.len() == 1 {
                    return Err("unexpected closing bracket".into());
                }
                let mut elems = frames.pop().ok_or("formula frame missing")?;
                let (mut tail, mut depth) =
                    elems.pop().ok_or("cell needs at least two elements")?;
                if elems.is_empty() {
                    return Err("cell needs at least two elements".into());
                }
                while let Some((head, head_depth)) = elems.pop() {
                    depth = 1 + depth.max(head_depth);
                    if depth > MAX_FORMULA_DEPTH {
                        return Err("formula depth limit".into());
                    }
                    charge_node(&mut nodes)?;
                    tail = r.pair(head, tail).ok_or(ARENA_FULL)?;
                }
                frames
                    .last_mut()
                    .ok_or("formula frame missing")?
                    .push((tail, depth));
                pos += 1;
            }
            b'0'..=b'9' => {
                let start = pos;
                while pos < bytes.len() && bytes[pos].is_ascii_digit() {
                    pos += 1;
                }
                let value = text[start..pos]
                    .parse::<u64>()
                    .map_err(|_| "invalid atom")?;
                charge_node(&mut nodes)?;
                let node = r.atom(Goldilocks::new(value)).ok_or(ARENA_FULL)?;
                frames
                    .last_mut()
                    .ok_or("formula frame missing")?
                    .push((node, 0));
            }
            _ => return Err(format!("invalid formula character at byte {pos}")),
        }
    }
    if frames.len() != 1 {
        return Err("unclosed '[' in formula".into());
    }
    let roots = frames.pop().ok_or("empty formula")?;
    if roots.len() != 1 {
        return Err("formula must contain exactly one noun".into());
    }
    Ok(roots[0].0)
}

fn charge_node(nodes: &mut usize) -> Result<(), String> {
    if *nodes == MAX_FORMULA_NODES {
        return Err("formula node limit".into());
    }
    *nodes += 1;
    Ok(())
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
/// Public words form `[word_last [... [word_first 0]]]`. Compiled source
/// entries validate the signature and reconstruct typed parameter nouns before
/// evaluating the body; raw nox formulas retain their own subject contract.
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
    if let Some(value) = r.atom_value(id) {
        return Ok(vec![value.as_u64()]);
    }
    let length = output_length(r, id)?;
    let mut out = Vec::new();
    out.try_reserve_exact(length)
        .map_err(|_| "output allocation failed")?;
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

/// Count each shared node once before expanding it. Native arena pairs only
/// reference earlier nodes, so that invariant also rejects invalid/cyclic data.
fn output_length<const N: usize>(r: &Reduction<N>, id: Order) -> Result<usize, String> {
    if r.get(id).is_none() {
        return Err("invalid data id in result".into());
    }
    let mut counts = vec![0usize; r.count() as usize];
    let mut pending = vec![id];
    while let Some(&node) = pending.last() {
        if counts[node as usize] != 0 {
            pending.pop();
        } else if r.atom_value(node).is_some() {
            counts[node as usize] = 1;
            pending.pop();
        } else {
            let head = r.head(node).ok_or("invalid result pair")?;
            let tail = r.tail(node).ok_or("invalid result pair")?;
            if head >= node || tail >= node {
                return Err("invalid result DAG ordering".into());
            }
            if counts[head as usize] == 0 {
                pending.push(head);
            } else if counts[tail as usize] == 0 {
                pending.push(tail);
            } else {
                let length = counts[head as usize] + counts[tail as usize];
                if length > MAX_OUTPUT_WORDS {
                    return Err("expanded output word limit".into());
                }
                counts[node as usize] = length;
                pending.pop();
            }
        }
    }
    Ok(counts[id as usize])
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
