#[test]
fn macro_diagnostics_are_reported_for_supported_misuses() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/integration/ui/*.rs");
}
