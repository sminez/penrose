#[test]
fn tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/stub/original-trait-maintained.rs");
    t.pass("tests/stub/stub-trait-is-generated.rs");
    t.pass("tests/stub/custom-prefix-accepted.rs");
    t.compile_fail("tests/stub/meta-args-reject-correctly.rs");
    t.pass("tests/stub/defaults-impls-are-copied.rs");
    t.compile_fail("tests/stub/bounds-are-maintained.rs");
    t.pass("tests/stub/stub-impls-satisfy-bounds.rs");
    t.pass("tests/stub/visibility-is-correct.rs");
    t.compile_fail("tests/stub/visibility-is-correct-failure.rs");
    t.compile_fail("tests/stub/test-only-attr-works.rs");
}
