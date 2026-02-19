#[test]
fn version_symbol_is_exposed() {
    let ptr = cudatrace::cudatrace_version();
    assert!(!ptr.is_null());
}
