//! Tests for `ifdef`/`ifndef`/`else`/`endif` conditional compilation.

use std::collections::HashSet;

use ziskasm::parser::parse_program_with_defines;

const SRC: &str = "\
main:
\tcopyb(0, 1) -> r5
ifdef ASM
\tcopyb(0, 2) -> r6
else
\tcopyb(0, 3) -> r7
endif
ifndef ASM
\tcopyb(0, 4) -> r8
endif
\tcopyb(0, 5) -> r9
";

fn verboses(defines: &[&str]) -> Vec<String> {
    let set: HashSet<String> = defines.iter().map(|s| s.to_string()).collect();
    parse_program_with_defines(SRC, "t", &set)
        .expect("parse")
        .instructions
        .iter()
        .map(|i| i.verbose.clone())
        .collect()
}

#[test]
fn without_define_takes_ifndef_and_else() {
    let v = verboses(&[]);
    // r5, else-branch r7, ifndef r8, r9
    assert_eq!(v.len(), 4, "{v:?}");
    assert!(v.iter().any(|s| s.contains("-> r7")), "else branch included");
    assert!(v.iter().any(|s| s.contains("-> r8")), "ifndef branch included");
    assert!(!v.iter().any(|s| s.contains("-> r6")), "ifdef branch excluded");
}

#[test]
fn with_define_takes_ifdef_skips_ifndef() {
    let v = verboses(&["ASM"]);
    // r5, ifdef r6, r9  (else and ifndef excluded)
    assert_eq!(v.len(), 3, "{v:?}");
    assert!(v.iter().any(|s| s.contains("-> r6")), "ifdef branch included");
    assert!(!v.iter().any(|s| s.contains("-> r7")), "else branch excluded");
    assert!(!v.iter().any(|s| s.contains("-> r8")), "ifndef branch excluded");
}

#[test]
fn unterminated_conditional_errors() {
    let set = HashSet::new();
    let err =
        parse_program_with_defines("main:\n\tcopyb(0,0)->r5\nifdef X\n", "t", &set).unwrap_err();
    assert!(err.contains("unterminated"), "{err}");
}

#[test]
fn stray_endif_errors() {
    let set = HashSet::new();
    let err =
        parse_program_with_defines("main:\n\tcopyb(0,0)->r5\nendif\n", "t", &set).unwrap_err();
    assert!(err.contains("endif"), "{err}");
}
