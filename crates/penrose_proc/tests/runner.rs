#[rustversion::stable]
const COMPILER: &str = "stable";

#[rustversion::beta]
const COMPILER: &str = "beta";

#[rustversion::nightly]
const COMPILER: &str = "nightly";

#[test]
fn tests() {
    let t = trybuild::TestCases::new();

    // Stubbed companion trait
    if COMPILER == "stable" {
        t.compile_fail("tests/stub/meta-args-reject-correctly.rs");
        t.compile_fail("tests/stub/test-only-attr-works.rs");
    }

    // Validate key bindings
    t.pass("tests/validate_bindings/valid-bindings-are-accepted.rs");
    t.pass("tests/validate_bindings/valid-template-bindings-are-accepted.rs");
    t.pass("tests/validate_bindings/templates-work-with-raw-bindings.rs");

    if COMPILER == "stable" {
        t.compile_fail("tests/validate_bindings/invalid-keys-are-rejected.rs");
        t.compile_fail("tests/validate_bindings/invalid-keys-with-modifiers-are-rejected.rs");
        t.compile_fail("tests/validate_bindings/invalid-modifiers-are-rejected.rs");
        t.compile_fail("tests/validate_bindings/keys-cannot-be-used-as-modifiers.rs");
        t.compile_fail("tests/validate_bindings/invalid-templates-are-rejected.rs");
        t.compile_fail("tests/validate_bindings/repeated-bindings-are-rejected.rs");
        t.compile_fail("tests/validate_bindings/bindings-clashing-with-templates-are-rejected.rs");
    }
}
