#[test]
fn tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/derive/original-trait-maintained.rs");
    t.pass("tests/derive/stub-trait-is-generated.rs");
    t.pass("tests/derive/custom-prefix-accepted.rs");
    t.compile_fail("tests/derive/meta-args-reject-correctly.rs");
    t.pass("tests/derive/defaults-impls-are-copied.rs");
}
