use super::common::{OsStrExt, TestDirectory, arg, zenvik};
use std::fs;
use std::process::Command;

#[test]
fn compiles_a_standalone_program_with_functions_and_collections() {
    let directory = TestDirectory::new();
    let source = directory.file("factorial and lists.zenvik");
    let executable = directory.file("standalone program");
    fs::write(
        &source,
        r#"
def factorial(number) {
    if number <= 1 {
        return 1;
    }
    return number * factorial(number - 1);
}
values = [1, 2, 3];
scores = {"answer": 5};
total = 0;
for value in values {
    total += value;
}
if total == 6 and scores["answer"] in [5] {
    print(factorial(5), total, values[-1], values[0:2]);
} else {
    print("wrong");
}
"#,
    )
    .unwrap();

    let compilation = zenvik(&[arg(&source), OsStrExt::as_os_str("-o"), arg(&executable)]);
    assert!(
        compilation.status.success(),
        "{}",
        String::from_utf8_lossy(&compilation.stderr)
    );
    assert!(executable.exists());
    let execution = Command::new(&executable).output().unwrap();
    assert!(execution.status.success());
    assert_eq!(
        String::from_utf8_lossy(&execution.stdout).trim(),
        "120 6 3 [1, 2]"
    );
}

#[test]
fn parses_dictionaries_with_entries_on_separate_lines() {
    let directory = TestDirectory::new();
    let source = directory.file("multiline-dictionary.zenvik");
    let executable = directory.file("multiline-dictionary");
    fs::write(
        &source,
        r#"
persona = {
    "edad": 17,
    "curso": 2
};
print(persona["edad"], persona["curso"]);
"#,
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
    assert_eq!(String::from_utf8_lossy(&execution.stdout).trim(), "17 2");
}

#[test]
fn checks_short_circuiting_and_runtime_exit_status() {
    let directory = TestDirectory::new();
    let source = directory.file("short-circuit.zenvik");
    let executable = directory.file("short-circuit");
    fs::write(
        &source,
        "if False and missing_name {\n    print(1);\n}\nprint(True or missing_name);\n",
    )
    .unwrap();
    let compilation = zenvik(&[arg(&source), OsStrExt::as_os_str("-o"), arg(&executable)]);
    assert!(compilation.status.success());
    let execution = Command::new(&executable).output().unwrap();
    assert!(execution.status.success());
    assert_eq!(String::from_utf8_lossy(&execution.stdout).trim(), "True");
}

#[test]
fn evaluates_operands_left_to_right_and_compares_types_correctly() {
    let directory = TestDirectory::new();
    let source = directory.file("evaluation.zenvik");
    let executable = directory.file("evaluation");
    fs::write(
        &source,
        r#"
def left() {
    print("left");
    return 8;
}
def right() {
    print("right");
    return 2;
}
print(left() / right());
print("1" == 1);
print(5.5 % 2.0);
"#,
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
        "left\nright\n4\nFalse\n1.5"
    );
}

#[test]
fn rejects_function_definitions_inside_top_level_control_blocks() {
    let directory = TestDirectory::new();
    let source = directory.file("nested-function.zenvik");
    fs::write(
        &source,
        "if True {\n    def hidden() {\n        return 1;\n    }\n}\n",
    )
    .unwrap();
    let output = zenvik(&[OsStrExt::as_os_str("-fsyntax-only"), arg(&source)]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("function definitions must be at top level")
    );
}
