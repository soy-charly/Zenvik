<<<<<<< HEAD
#[path = "compiler/cli.rs"]
mod cli;
#[path = "compiler/common.rs"]
mod common;
#[path = "compiler/imports.rs"]
mod imports;
#[path = "compiler/language.rs"]
mod language;
#[path = "compiler/numbers.rs"]
mod numbers;
=======
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "zenvik-e2e-{}-{sequence}-with spaces",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn zenvik(arguments: &[&std::ffi::OsStr]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_zenvik"))
        .args(arguments)
        .output()
        .unwrap()
}

fn arg(path: &std::path::Path) -> &std::ffi::OsStr {
    path.as_os_str()
}

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
    let execution = Command::new(executable).output().unwrap();
    assert!(execution.status.success());
    assert_eq!(
        String::from_utf8_lossy(&execution.stdout).trim(),
        "333333283333335000000"
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
fn supports_c_and_cpp_scalar_types_and_typed_functions() {
    let directory = TestDirectory::new();
    let source = directory.file("typed.zenvik");
    let executable = directory.file("typed");
    fs::write(
        &source,
        r#"
int add(int left, unsigned short right) {
    return left + right;
}
double half(double value) {
    return value / 2;
}
void greet(string message) {
    print(message);
    return;
}
void noop(void) {
    return;
}
int total = add(3, 5);
float average = half(total);
char letter = "Z";
unsigned long long count = 42;
bool ready = True;
int narrowed = 1;
int32_t native = 9;
narrowed = 2.9;
print(total, average, letter, count, ready, narrowed, native);
greet("typed");
noop();
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
        "8 4 Z 42 True 2 9\ntyped"
    );
}

#[test]
fn supports_string_type_without_std_prefix() {
    let directory = TestDirectory::new();
    let source = directory.file("string-type.zenvik");
    let executable = directory.file("string-type");
    fs::write(&source, "string greeting = \"hello\";\nprint(greeting);\n").unwrap();

    let compilation = zenvik(&[arg(&source), OsStrExt::as_os_str("-o"), arg(&executable)]);
    assert!(
        compilation.status.success(),
        "{}",
        String::from_utf8_lossy(&compilation.stderr)
    );
    let execution = Command::new(executable).output().unwrap();
    assert!(execution.status.success());
    assert_eq!(String::from_utf8_lossy(&execution.stdout).trim(), "hello");
}

#[test]
fn accepts_c_fundamental_and_standard_integer_alias_types() {
    let directory = TestDirectory::new();
    let source = directory.file("scalar-types.zenvik");
    fs::write(
        &source,
        r#"
signed char signed_char = "a";
unsigned char unsigned_char = "b";
short short_value = 1;
unsigned short unsigned_short = 1;
signed int signed_int = 1;
unsigned int unsigned_int = 1;
long long_value = 1;
unsigned long unsigned_long = 1;
long long long_long = 1;
unsigned long long unsigned_long_long = 1;
float float_value = 1.0;
long double long_double = 1.0;
wchar_t wide_char = "c";
char8_t char8 = "d";
char16_t char16 = "e";
char32_t char32 = "f";
int8_t i8 = 1;
uint8_t u8 = 1;
int16_t i16 = 1;
uint16_t u16 = 1;
int32_t i32 = 1;
uint32_t u32 = 1;
int64_t i64 = 1;
uint64_t u64 = 1;
intptr_t ip = 1;
uintptr_t up = 1;
intmax_t im = 1;
uintmax_t um = 1;
size_t size = 1;
ptrdiff_t difference = 1;
size_t standard_size = 1;
"#,
    )
    .unwrap();
    let output = zenvik(&[OsStrExt::as_os_str("-fsyntax-only"), arg(&source)]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn rejects_incompatible_inferred_and_declared_assignments() {
    let directory = TestDirectory::new();
    let source = directory.file("type-error.zenvik");
    fs::write(&source, "int amount = 3;\namount = \"three\";\n").unwrap();
    let output = zenvik(&[OsStrExt::as_os_str("-fsyntax-only"), arg(&source)]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("cannot assign `string` to `integer` variable `amount`")
    );
}

#[test]
fn checks_typed_function_arguments_and_return_values() {
    let directory = TestDirectory::new();
    let source = directory.file("typed-function-error.zenvik");
    fs::write(
        &source,
        "int identity(int value) {\n    return value;\n}\nprint(identity(\"wrong\"));\n",
    )
    .unwrap();
    let output = zenvik(&[OsStrExt::as_os_str("-fsyntax-only"), arg(&source)]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("cannot assign `string` to `integer` variable `argument 1`")
    );

    fs::write(
        &source,
        "int identity(int value) {\n    return \"wrong\";\n}\n",
    )
    .unwrap();
    let output = zenvik(&[OsStrExt::as_os_str("-fsyntax-only"), arg(&source)]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("cannot assign `string` to `integer` variable `return value`")
    );
}

#[test]
fn enforces_ranges_for_fixed_width_integer_types() {
    let directory = TestDirectory::new();
    let source = directory.file("integer-range.zenvik");
    let executable = directory.file("integer-range");
    fs::write(&source, "int8_t value = 128;\nprint(value);\n").unwrap();
    let compilation = zenvik(&[arg(&source), OsStrExt::as_os_str("-o"), arg(&executable)]);
    assert!(
        compilation.status.success(),
        "{}",
        String::from_utf8_lossy(&compilation.stderr)
    );
    let execution = Command::new(executable).output().unwrap();
    assert_eq!(execution.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&execution.stderr).contains("out of range for type `int8_t`"));
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

struct OsStrExt;
impl OsStrExt {
    fn as_os_str(value: &str) -> &std::ffi::OsStr {
        std::ffi::OsStr::new(value)
    }
}
>>>>>>> cfc282f (added standard libraries)
