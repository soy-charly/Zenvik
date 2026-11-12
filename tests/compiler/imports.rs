use super::common::{OsStrExt, TestDirectory, arg, zenvik};
use std::fs;
use std::process::Command;

#[test]
fn compiles_multiple_source_files_together() {
    let directory = TestDirectory::new();
    let library = directory.file("math.zenvik");
    let program = directory.file("main.zenvik");
    let executable = directory.file("multiple");
    fs::write(&library, "def double(value) {\n    return value * 2;\n}\n").unwrap();
    fs::write(&program, "print(double(21));\n").unwrap();
    let compilation = zenvik(&[
        arg(&program),
        arg(&library),
        OsStrExt::as_os_str("-o"),
        arg(&executable),
    ]);
    assert!(
        compilation.status.success(),
        "{}",
        String::from_utf8_lossy(&compilation.stderr)
    );
    let execution = Command::new(executable).output().unwrap();
    assert!(execution.status.success());
    assert_eq!(String::from_utf8_lossy(&execution.stdout).trim(), "42");
}

#[test]
fn imports_relative_modules_transitively_and_only_once() {
    let directory = TestDirectory::new();
    let modules = directory.file("module sources");
    fs::create_dir_all(modules.join("lib")).unwrap();
    let common = modules.join("common.zenvik");
    let library = modules.join("lib/tools.zenvik");
    let source = modules.join("main.zenvik");
    let executable = directory.file("imported-program");
    fs::write(
        &common,
        "print(\"common loaded\");\ndef twice(value) {\n    return value * 2;\n}\n",
    )
    .unwrap();
    fs::write(
        &library,
        "import \"../common.zenvik\";\ndef add_one(value) {\n    return value + 1;\n}\n",
    )
    .unwrap();
    fs::write(
        &source,
        "import \"lib/tools.zenvik\";\nimport \"common.zenvik\";\nprint(twice(add_one(20)));\n",
    )
    .unwrap();

    let compilation = zenvik(&[arg(&source), OsStrExt::as_os_str("-o"), arg(&executable)]);
    assert!(
        compilation.status.success(),
        "{}",
        String::from_utf8_lossy(&compilation.stderr)
    );
    let execution = Command::new(executable).output().unwrap();
    assert!(execution.status.success());
    assert_eq!(
        String::from_utf8_lossy(&execution.stdout).trim(),
        "common loaded\n42"
    );
}

#[test]
fn imports_repository_and_configured_libraries_by_name() {
    let directory = TestDirectory::new();
    let source = directory.file("standard-library.zenvik");
    let executable = directory.file("standard-library");
    fs::write(&source, "import math;\nprint(square(9));\n").unwrap();

    let compilation = zenvik(&[arg(&source), OsStrExt::as_os_str("-o"), arg(&executable)]);
    assert!(
        compilation.status.success(),
        "{}",
        String::from_utf8_lossy(&compilation.stderr)
    );
    let execution = Command::new(&executable).output().unwrap();
    assert!(execution.status.success());
    assert_eq!(String::from_utf8_lossy(&execution.stdout).trim(), "81");

    let custom_libraries = directory.file("custom libraries");
    fs::create_dir_all(&custom_libraries).unwrap();
    fs::write(
        custom_libraries.join("custom.zenvik"),
        "def answer() {\n    return 42;\n}\n",
    )
    .unwrap();
    fs::write(&source, "import custom;\nprint(answer());\n").unwrap();
    let compilation = zenvik(&[
        OsStrExt::as_os_str("-L"),
        arg(&custom_libraries),
        arg(&source),
        OsStrExt::as_os_str("-o"),
        arg(&executable),
    ]);
    assert!(
        compilation.status.success(),
        "{}",
        String::from_utf8_lossy(&compilation.stderr)
    );
    let execution = Command::new(executable).output().unwrap();
    assert!(execution.status.success());
    assert_eq!(String::from_utf8_lossy(&execution.stdout).trim(), "42");
}

#[test]
fn reports_import_cycles_and_rejects_nested_imports() {
    let directory = TestDirectory::new();
    let first = directory.file("first.zenvik");
    let second = directory.file("second.zenvik");
    let missing = directory.file("missing-import.zenvik");
    fs::write(&missing, "import \"not-found.zenvik\";\n").unwrap();
    let output = zenvik(&[arg(&missing)]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot import `not-found.zenvik`"));

    fs::write(&missing, "import not_found;\n").unwrap();
    let output = zenvik(&[arg(&missing)]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot find library `not_found`"));

    fs::write(&first, "import \"second.zenvik\";\n").unwrap();
    fs::write(&second, "import \"first.zenvik\";\n").unwrap();
    let output = zenvik(&[arg(&first)]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("import cycle"));

    fs::write(&first, "if True {\n    import \"second.zenvik\";\n}\n").unwrap();
    let output = zenvik(&[OsStrExt::as_os_str("-fsyntax-only"), arg(&first)]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("imports are only supported at top level")
    );
}
