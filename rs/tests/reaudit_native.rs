//! Independent native-runtime re-audit probes, September 2026.
use joy_rs::Warrior;
use trident::runtime::{artifact::BundleCost, ProgramBundle, ProgramInput, Runner};

fn bundle(assembly: String) -> ProgramBundle {
    ProgramBundle {
        name: "reaudit".into(),
        version: "1".into(),
        target_vm: "nox".into(),
        target_os: None,
        source_hash: "reaudit-source".into(),
        assembly,
        entry_point: "main".into(),
        functions: vec![],
        reads_state: false,
        cost: BundleCost {
            table_values: vec![],
            table_names: vec![],
            padded_height: 0,
            estimated_proving_ns: 0,
        },
    }
}

fn input() -> ProgramInput {
    ProgramInput {
        public: vec![],
        secret: vec![],
        digests: vec![],
    }
}

#[test]
fn shared_result_expansion_is_checked_before_flat_output_allocation() {
    // Twenty applications of duplicate-subject produce a million logical
    // leaves while allocating only one new shared pair per application.
    let mut formula = "[1 42]".to_string();
    for _ in 0..20 {
        formula = format!("[2 [{formula} [1 [3 [[0 1] [0 1]]]]]]");
    }
    assert_eq!(formula.len(), 566);
    let result = Warrior::with_budget(101)
        .run(&bundle(formula.clone()), &input())
        .unwrap();
    assert_eq!(result.cycle_count, 101);
    assert_eq!(result.output.len(), 1 << 20);
    assert!(result.output.iter().all(|&word| word == 42));
    let formula = format!("[2 [{formula} [1 [3 [[0 1] [0 1]]]]]]");
    assert_eq!(
        Warrior::with_budget(106)
            .run(&bundle(formula), &input())
            .unwrap_err(),
        "expanded output word limit"
    );
}

#[test]
fn malformed_raw_formula_returns_an_error_without_process_abort() {
    if std::env::var_os("JOY_REAUDIT_DEEP_PARSE_CHILD").is_some() {
        let program = bundle("[".repeat(2_000_000));
        assert_eq!(
            Warrior::with_budget(1).run(&program, &input()).unwrap_err(),
            "formula depth limit"
        );
        return;
    }
    // Isolate a possible stack overflow: it aborts its process, not just the
    // reduce thread, and cannot be contained by JoinHandle::join.
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "malformed_raw_formula_returns_an_error_without_process_abort",
            "--nocapture",
        ])
        .env("JOY_REAUDIT_DEEP_PARSE_CHILD", "1")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let started = std::time::Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if started.elapsed() >= std::time::Duration::from_secs(5) {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("raw parser exceeded five seconds before rejecting a malformed formula");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    };
    assert!(status.success(), "raw parser child failed: {status}");
}

#[test]
fn binary_and_nary_formulas_share_the_noun_depth_ceiling() {
    use joy_rs::formula::{parse, MAX_FORMULA_DEPTH};
    let mut arena = nox::Reduction::<{ 1 << 14 }>::try_new_boxed().unwrap();
    let binary = format!(
        "{}0{}",
        "[0 ".repeat(MAX_FORMULA_DEPTH),
        "]".repeat(MAX_FORMULA_DEPTH)
    );
    let nary = format!("[{}0]", "0 ".repeat(MAX_FORMULA_DEPTH));
    let binary_root = parse(&mut arena, &binary).unwrap();
    assert_eq!(parse(&mut arena, &nary).unwrap(), binary_root);
    assert_eq!(
        parse(&mut arena, &format!("[0 {binary}]")).unwrap_err(),
        "formula depth limit"
    );
    let nary = format!("[{}0]", "0 ".repeat(MAX_FORMULA_DEPTH + 1));
    assert_eq!(parse(&mut arena, &nary).unwrap_err(), "formula depth limit");
}

#[test]
fn hash_consing_does_not_bypass_the_syntactic_node_ceiling() {
    use joy_rs::formula::{parse, MAX_FORMULA_NODES};
    let mut arena = nox::Reduction::<1024>::try_new_boxed().unwrap();
    let repeated = format!("[{}]", "[0 0] ".repeat(MAX_FORMULA_NODES / 3 + 1));
    assert_eq!(
        parse(&mut arena, &repeated).unwrap_err(),
        "formula node limit"
    );
    assert_eq!(
        arena.count(),
        2,
        "physical DAG stays tiny while syntax grows"
    );
}

#[test]
fn dynamic_self_application_hits_the_native_depth_guard() {
    let again = "[2 [[0 1] [0 1]]]";
    let program = bundle(format!("[2 [[1 {again}] [1 {again}]]]"));
    assert_eq!(
        Warrior::new().run(&program, &input()).unwrap_err(),
        "reduction error: malformed formula"
    );
}
