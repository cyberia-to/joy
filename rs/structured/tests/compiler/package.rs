use super::*;
use std::{fs, path::PathBuf};

struct Directory(PathBuf);
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn packed_collections_match_independent_reference_across_word_and_tree_boundaries() {
    let dir =
        Directory(std::env::temp_dir().join(format!("joy-package-model-{}", std::process::id())));
    fs::create_dir(&dir.0).unwrap();
    for length in [0usize, 1, 2, 3, 4, 5, 7, 8, 9, 65, 129, 257, 513] {
        let mut ar = Arena::new();
        let generated = generated_program(&mut ar);
        let compiler = compiler(&mut ar, generated, 0, None);
        fs::write(dir.0.join("compiler"), encoded(&ar, compiler)).unwrap();
        let (mut modules, mut options) = configuration();
        options.cfg = vec!["a".into(), "b".into(), "release".into()];
        modules[0].source = (0..length).map(|i| (i * 37) as u8).collect();
        let mut unused = modules[0].clone();
        unused.path = "unused".into();
        unused.source = vec![0; length];
        modules.push(unused);
        let mut empty = modules[0].clone();
        empty.path = "zempty".into();
        empty.source.clear();
        modules.push(empty);
        let expected = schema::job(
            &mut ar,
            compiler,
            &modules,
            "demo",
            "main",
            &options,
            &schema::FIXTURE_CAPS,
        )
        .unwrap();
        for (i, m) in modules.iter().enumerate() {
            fs::write(dir.0.join(format!("source{i}")), &m.source).unwrap();
        }
        let manifest = serde_json::json!({"version":1,"entry_module":"demo","entry_function":"main",
            "modules":modules.iter().enumerate().rev().map(|(i,m)|serde_json::json!({
                "logical_path":m.path,"file":format!("source{i}"),"origin_name":m.origin,"origin_version":m.version})).collect::<Vec<_>>(),
            "options":{"target":0,"input_profile":0,"output_profile":0,"optimization":0,"cfg_flags":options.cfg.iter().rev().collect::<Vec<_>>()},
            "limits":{"source_bytes":4096,"modules":32,"diagnostics":16,"sequence_length":4096,"validation_visits":100000,
                "artifact_bytes":1048576,"artifact_nodes":3000,"artifact_depth":128,"reductions":1000000,"arena_nodes":3000,"evaluator_frames":16384}});
        fs::write(
            dir.0.join("package.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        let packed = pack_job_files(
            &dir.0.join("compiler"),
            &dir.0.join("package.json"),
            RunLimits::default(),
        )
        .unwrap();
        assert_eq!(packed.bytes, encoded(&ar, expected), "length {length}");
        assert_eq!(packed.report.job_particle, particle(&ar, expected).unwrap());
    }
}
