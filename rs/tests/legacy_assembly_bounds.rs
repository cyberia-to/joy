use joy_rs::ArtifactMeta;

fn metadata() -> ArtifactMeta {
    ArtifactMeta {
        program: String::new(),
        output: vec![],
        cycle_count: 0,
        assembly: None,
        assembly_deflate: None,
    }
}

#[test]
fn compressed_legacy_assembly_cannot_expand_past_the_formula_limit() {
    let mut meta = metadata();
    let boundary = " ".repeat(256 * 1024);
    meta.set_assembly(&boundary);
    assert_eq!(meta.assembly_text().unwrap().unwrap(), boundary);
    meta.set_assembly(&(boundary.clone() + " "));
    assert!(meta.assembly_deflate.as_ref().unwrap().len() < 1024);
    assert!(meta.assembly_text().unwrap_err().contains("oversized"));
    meta.assembly_deflate = None;
    meta.assembly = Some(boundary + " ");
    assert!(meta.assembly_text().unwrap_err().contains("size limit"));
    meta.set_assembly("[1 7]");
    assert_eq!(meta.assembly_text().unwrap().as_deref(), Some("[1 7]"));
}
