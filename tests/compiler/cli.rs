use super::common::{OsStrExt, TestDirectory, arg, zenvik};
use std::fs;

#[test]
fn supports_syntax_only_object_and_assembly_modes() {
    let directory = TestDirectory::new();
    let source = directory.file("simple.zenvik");
    fs::write(&source, "print(2 + 3);\n").unwrap();
    assert!(
        zenvik(&[OsStrExt::as_os_str("-fsyntax-only"), arg(&source)])
            .status
            .success()
    );

    let object = directory.file("simple.o");
    assert!(
        zenvik(&[
            OsStrExt::as_os_str("-c"),
            arg(&source),
            OsStrExt::as_os_str("-o"),
            arg(&object)
        ])
        .status
        .success()
    );
    assert!(object.exists());

    let assembly = directory.file("simple.s");
    assert!(
        zenvik(&[
            OsStrExt::as_os_str("-S"),
            arg(&source),
            OsStrExt::as_os_str("-o"),
            arg(&assembly)
        ])
        .status
        .success()
    );
    assert!(assembly.exists());
}

#[test]
fn reports_source_locations_and_unknown_functions() {
    let directory = TestDirectory::new();
    let source = directory.file("bad.zenvik");
    fs::write(&source, "\nprint(missing_function());\n").unwrap();
    let output = zenvik(&[arg(&source)]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("bad.zenvik:2:7: error: unknown function `missing_function`"));
}

#[test]
fn requires_semicolons_after_simple_statements() {
    let directory = TestDirectory::new();
    let source = directory.file("missing-semicolon.zenvik");
    fs::write(&source, "value = 1\nprint(value);\n").unwrap();
    let output = zenvik(&[arg(&source)]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(
        "missing-semicolon.zenvik:1:10: error: expected `;` at the end of the statement"
    ));

    fs::write(
        &source,
        "value = 1;\nif value == 1 {\n    print(value);\n}\n",
    )
    .unwrap();
    let output = zenvik(&[OsStrExt::as_os_str("-fsyntax-only"), arg(&source)]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
