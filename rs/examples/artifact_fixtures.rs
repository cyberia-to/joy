//! Hand-authored CLI golden fixtures. No compiler or proof claim.
use nebu::Goldilocks;
use nox::{artifact, Order, Reduction};
use std::path::PathBuf;

type Arena = Reduction<256>;
type Result<T> = std::result::Result<T, String>;
fn atom(ar: &mut Arena, value: u64) -> Result<Order> {
    ar.atom(Goldilocks::new(value)).ok_or("arena full".into())
}
fn pair(ar: &mut Arena, a: Order, b: Order) -> Result<Order> {
    ar.pair(a, b).ok_or("arena full".into())
}
fn op(ar: &mut Arena, tag: u64, body: Order) -> Result<Order> {
    let tag = atom(ar, tag)?;
    pair(ar, tag, body)
}
fn binary(ar: &mut Arena, tag: u64, a: Order, b: Order) -> Result<Order> {
    let body = pair(ar, a, b)?;
    op(ar, tag, body)
}
fn loop_fixture(ar: &mut Arena) -> Result<(Order, Order, Order)> {
    let zero = atom(ar, 0)?;
    let one = atom(ar, 1)?;
    let two = atom(ar, 2)?;
    let six = atom(ar, 6)?;
    let seven = atom(ar, 7)?;
    let q0 = op(ar, 1, zero)?;
    let q1 = op(ar, 1, one)?;
    let code = op(ar, 0, two)?;
    let count = op(ar, 0, six)?;
    let acc = op(ar, 0, seven)?;
    let done = binary(ar, 9, count, q0)?;
    let next = binary(ar, 6, count, q1)?;
    let next_acc = binary(ar, 5, acc, q1)?;
    let state = binary(ar, 3, next, next_acc)?;
    let subject = binary(ar, 3, code, state)?;
    let again = binary(ar, 2, subject, code)?;
    let arms = pair(ar, acc, again)?;
    let formula = binary(ar, 4, done, arms)?;
    let n = atom(ar, 4097)?;
    let initial = pair(ar, n, zero)?;
    Ok((formula, pair(ar, formula, initial)?, n))
}
fn encode(ar: &Arena, root: Order) -> Result<Vec<u8>> {
    artifact::encode(
        ar,
        root,
        artifact::Limits {
            max_bytes: 16384,
            max_nodes: 192,
            max_depth: 128,
        },
    )
    .map_err(|e| format!("{e:?}"))
}
fn vectors() -> Result<String> {
    let mut out = Vec::new();
    for name in ["add14", "identity_tree", "loop4097"] {
        let mut ar = Arena::new();
        let zero = atom(&mut ar, 0)?;
        let (formula, input, expected) = if name == "add14" {
            let seven = atom(&mut ar, 7)?;
            let q = op(&mut ar, 1, seven)?;
            let body = pair(&mut ar, q, q)?;
            (op(&mut ar, 5, body)?, zero, atom(&mut ar, 14)?)
        } else if name == "loop4097" {
            loop_fixture(&mut ar)?
        } else {
            let a = atom(&mut ar, 1)?;
            let b = atom(&mut ar, 2)?;
            let c = atom(&mut ar, 3)?;
            let ab = pair(&mut ar, a, b)?;
            let input = pair(&mut ar, ab, c)?;
            (op(&mut ar, 0, a)?, input, input)
        };
        let mut rest = zero;
        for field in [zero, zero, zero, formula].into_iter().rev() {
            rest = pair(&mut ar, field, rest)?;
        }
        let program = op(&mut ar, 0x41525431, rest)?;
        out.push(
            serde_json::json!({ "name":name,"program":encode(&ar,program)?,
            "input":encode(&ar,input)?,"expected_output":encode(&ar,expected)? }),
        );
    }
    serde_json::to_string(&out)
        .map(|s| s + "\n")
        .map_err(|e| e.to_string())
}
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let path = PathBuf::from(
        args.next()
            .ok_or("usage: artifact_fixtures OUTPUT [--check]")?,
    );
    let check = match args.next() {
        None => false,
        Some(v) if v == "--check" => true,
        _ => return Err("unexpected argument".into()),
    };
    if args.next().is_some() {
        return Err("too many arguments".into());
    }
    let json = vectors()?;
    if check {
        if std::fs::read_to_string(path)? != json {
            return Err("fixture vectors stale".into());
        }
    } else {
        std::fs::write(path, json)?;
    }
    Ok(())
}
