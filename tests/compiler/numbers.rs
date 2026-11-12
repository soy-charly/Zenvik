use super::common::{OsStrExt, TestDirectory, arg, zenvik};
use std::fs;
use std::process::Command;

#[test]
fn preserves_precision_for_integer_sums_beyond_64_bits() {
    let directory = TestDirectory::new();
    let source = directory.file("quadratic-sum.zenvik");
    let executable = directory.file("quadratic-sum");
    fs::write(
        &source,
        r#"
def quadratic_sum(n) {
    total = 0;
    for i in range(n) {
        total = total + (i * i);
    }
    return total;
}
print(quadratic_sum(10000000));
"#,
    )
    .unwrap();

    let compilation = zenvik(&[arg(&source), OsStrExt::as_os_str("-o"), arg(&executable)]);
    assert!(
        compilation.status.success(),
        "{}",
        String::from_utf8_lossy(&compilation.stderr)
    );
    let execution = Command::new(&executable).output().unwrap();
    assert!(execution.status.success());
    assert_eq!(
        String::from_utf8_lossy(&execution.stdout).trim(),
        "333333283333335000000"
    );
}

#[test]
fn evaluates_power_with_correct_precedence_associativity_and_integer_precision() {
    let directory = TestDirectory::new();
    let source = directory.file("power.zenvik");
    let executable = directory.file("power");
    fs::write(
        &source,
        "print(-2 ** 2);\nprint(2 ** 3 ** 2);\nprint(3 ** 40);\nprint(2 ** -2);\n",
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
        "-4\n512\n12157665459056928801\n0.25"
    );
}

#[test]
fn compares_large_integers_to_floats_without_losing_precision() {
    let directory = TestDirectory::new();
    let source = directory.file("mixed-numeric-comparisons.zenvik");
    let executable = directory.file("mixed-numeric-comparisons");
    fs::write(
        &source,
        "print(9007199254740993 == 9007199254740992.0);\nprint(9007199254740993 > 9007199254740992.0);\nprint(9007199254740991 < 9007199254740992.0);\n",
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
        "False\nTrue\nTrue"
    );
}

#[test]
fn rejects_trailing_characters_in_numeric_string_conversions() {
    let directory = TestDirectory::new();
    let source = directory.file("invalid-conversion.zenvik");
    let executable = directory.file("invalid-conversion");
    fs::write(&source, "print(int(\"12xyz\"));\n").unwrap();
    let compilation = zenvik(&[arg(&source), OsStrExt::as_os_str("-o"), arg(&executable)]);
    assert!(
        compilation.status.success(),
        "{}",
        String::from_utf8_lossy(&compilation.stderr)
    );
    let execution = Command::new(&executable).output().unwrap();
    assert_eq!(execution.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&execution.stderr).contains("invalid integer conversion"));

    fs::write(&source, "print(float(\"3.5tail\"));\n").unwrap();
    let compilation = zenvik(&[arg(&source), OsStrExt::as_os_str("-o"), arg(&executable)]);
    assert!(compilation.status.success());
    let execution = Command::new(&executable).output().unwrap();
    assert_eq!(execution.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&execution.stderr).contains("invalid numeric conversion"));
}

#[test]
fn preserves_nul_followed_by_octal_digit_in_generated_cpp_strings() {
    let directory = TestDirectory::new();
    let source = directory.file("nul-string.zenvik");
    let executable = directory.file("nul-string");
    fs::write(&source, "print(len(\"\\07\"));\n").unwrap();
    let compilation = zenvik(&[arg(&source), OsStrExt::as_os_str("-o"), arg(&executable)]);
    assert!(
        compilation.status.success(),
        "{}",
        String::from_utf8_lossy(&compilation.stderr)
    );
    let execution = Command::new(executable).output().unwrap();
    assert!(execution.status.success());
    assert_eq!(String::from_utf8_lossy(&execution.stdout).trim(), "2");
}
