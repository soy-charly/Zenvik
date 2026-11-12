mod ast;
mod codegen;
mod lexer;
mod parser;
mod sema;

use std::collections::HashSet;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Copy, PartialEq)]
enum OutputMode {
    Executable,
    Object,
    Assembly,
    SyntaxOnly,
}

struct Options {
    inputs: Vec<PathBuf>,
    library_paths: Vec<PathBuf>,
    output: Option<PathBuf>,
    mode: OutputMode,
    optimization: Option<String>,
    verbose: bool,
}

fn main() {
    let code = match run(env::args_os().skip(1).collect()) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("zenvik: error: {error}");
            2
        }
    };
    process::exit(code);
}

fn run(arguments: Vec<OsString>) -> Result<i32, String> {
    if arguments.is_empty() {
        print_help();
        return Ok(0);
    }
    let options = parse_arguments(arguments)?;
    let library_paths = library_search_paths(&options);
    let mut loaded = HashSet::new();
    let mut active = Vec::new();
    let mut statements = Vec::new();
    for path in &options.inputs {
        statements.extend(load_source(
            path,
            None,
            &library_paths,
            &mut loaded,
            &mut active,
        )?);
    }
    sema::check(&statements)?;
    let generated = codegen::generate(&statements);
    if options.mode == OutputMode::SyntaxOnly {
        return compile(&generated, None, &options);
    }
    let output = options
        .output
        .clone()
        .unwrap_or_else(|| default_output(&options.inputs[0], options.mode));
    compile(&generated, Some(&output), &options)
}

fn load_source(
    path: &Path,
    import_site: Option<(&ast::Position, &str)>,
    library_paths: &[PathBuf],
    loaded: &mut HashSet<PathBuf>,
    active: &mut Vec<PathBuf>,
) -> Result<Vec<ast::Stmt>, String> {
    let canonical = fs::canonicalize(path).map_err(|error| {
        if let Some((position, imported)) = import_site {
            format!(
                "{}:{}:{}: error: cannot import `{imported}`: {error}",
                position.file, position.line, position.column
            )
        } else {
            format!("cannot read {}: {error}", path.display())
        }
    })?;
    if let Some(start) = active.iter().position(|current| current == &canonical) {
        let cycle = active[start..]
            .iter()
            .chain(std::iter::once(&canonical))
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(" -> ");
        let (position, imported) =
            import_site.expect("only imported files can form an active cycle");
        return Err(format!(
            "{}:{}:{}: error: import cycle while importing `{imported}`: {cycle}",
            position.file, position.line, position.column
        ));
    }
    if !loaded.insert(canonical.clone()) {
        return Ok(Vec::new());
    }

    active.push(canonical.clone());
    let result = (|| {
        let source = fs::read_to_string(path).map_err(|error| {
            if let Some((position, imported)) = import_site {
                format!(
                    "{}:{}:{}: error: cannot read import `{imported}`: {error}",
                    position.file, position.line, position.column
                )
            } else {
                format!("cannot read {}: {error}", path.display())
            }
        })?;
        let tokens = lexer::lex(&source, &path.display().to_string())?;
        let statements = parser::parse(tokens)?;
        let mut expanded = Vec::new();
        for statement in statements {
            if let ast::StmtKind::Import(target) = &statement.kind {
                let imported = match target {
                    ast::ImportTarget::Path(imported) => path
                        .parent()
                        .unwrap_or_else(|| Path::new("."))
                        .join(imported),
                    ast::ImportTarget::Library(name) => {
                        resolve_library(name, library_paths, &statement.position)?
                    }
                };
                let display_name = match target {
                    ast::ImportTarget::Path(imported) => imported.as_str(),
                    ast::ImportTarget::Library(name) => name.as_str(),
                };
                expanded.extend(load_source(
                    &imported,
                    Some((&statement.position, display_name)),
                    library_paths,
                    loaded,
                    active,
                )?);
            } else {
                expanded.push(statement);
            }
        }
        Ok(expanded)
    })();
    active.pop();
    if result.is_err() {
        loaded.remove(&canonical);
    }
    result
}

fn resolve_library(
    name: &str,
    library_paths: &[PathBuf],
    position: &ast::Position,
) -> Result<PathBuf, String> {
    for directory in library_paths {
        let candidate = directory.join(format!("{name}.zenvik"));
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    let searched = library_paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    Err(format!(
        "{}:{}:{}: error: cannot find library `{name}`; searched: {searched}",
        position.file, position.line, position.column
    ))
}

fn library_search_paths(options: &Options) -> Vec<PathBuf> {
    let mut paths = options.library_paths.clone();
    if let Some(environment_paths) = env::var_os("ZENVIK_LIB_DIR") {
        paths.extend(env::split_paths(&environment_paths));
    }
    paths.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("libraries"));
    paths.push(PathBuf::from("/usr/local/share/zenvik/libraries"));
    paths
}

fn parse_arguments(arguments: Vec<OsString>) -> Result<Options, String> {
    let mut inputs = Vec::new();
    let mut library_paths = Vec::new();
    let mut output = None;
    let mut mode = OutputMode::Executable;
    let mut mode_was_set = false;
    let mut optimization = None;
    let mut verbose = false;
    let mut index = 0;
    while index < arguments.len() {
        let argument = arguments[index].to_string_lossy();
        match argument.as_ref() {
            "--help" | "-h" => {
                print_help();
                process::exit(0);
            }
            "--version" => {
                println!("zenvik {VERSION}");
                process::exit(0);
            }
            "-o" => {
                index += 1;
                let path = arguments
                    .get(index)
                    .ok_or_else(|| "`-o` requires an output path".to_owned())?;
                if output.replace(PathBuf::from(path)).is_some() {
                    return Err("`-o` may only be specified once".into());
                }
            }
            "-L" | "--library-path" => {
                index += 1;
                let path = arguments
                    .get(index)
                    .ok_or_else(|| format!("`{argument}` requires a directory"))?;
                library_paths.push(PathBuf::from(path));
            }
            "-c" | "-S" | "-fsyntax-only" => {
                if mode_was_set {
                    return Err("choose only one of `-c`, `-S`, and `-fsyntax-only`".into());
                }
                mode_was_set = true;
                mode = match argument.as_ref() {
                    "-c" => OutputMode::Object,
                    "-S" => OutputMode::Assembly,
                    _ => OutputMode::SyntaxOnly,
                };
            }
            "-O0" | "-O1" | "-O2" | "-O3" => {
                if optimization.replace(argument.to_string()).is_some() {
                    return Err("specify only one optimization level".into());
                }
            }
            "-v" => verbose = true,
            _ if argument.starts_with('-') => {
                return Err(format!("unknown option `{argument}`"));
            }
            _ => inputs.push(PathBuf::from(&arguments[index])),
        }
        index += 1;
    }
    if inputs.is_empty() {
        return Err("no input files; try `zenvik --help`".into());
    }
    if mode == OutputMode::SyntaxOnly && output.is_some() {
        return Err("`-o` cannot be used with `-fsyntax-only`".into());
    }
    Ok(Options {
        inputs,
        library_paths,
        output,
        mode,
        optimization,
        verbose,
    })
}

fn default_output(input: &Path, mode: OutputMode) -> PathBuf {
    let mut path = input.with_extension("");
    match mode {
        OutputMode::Executable => {}
        OutputMode::Object => {
            path.set_extension("o");
        }
        OutputMode::Assembly => {
            path.set_extension("s");
        }
        OutputMode::SyntaxOnly => unreachable!(),
    };
    path
}

fn compile(generated: &str, output: Option<&Path>, options: &Options) -> Result<i32, String> {
    let compiler = env::var_os("ZENVIK_CXX").unwrap_or_else(|| OsString::from("clang++"));
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock error: {error}"))?
        .as_nanos();
    let source_path = env::temp_dir().join(format!("zenvik-{}-{nonce}.cpp", process::id()));
    fs::write(&source_path, generated)
        .map_err(|error| format!("cannot write temporary backend source: {error}"))?;

    let mut command = Command::new(&compiler);
    command.arg("-std=c++20");
    if let Some(level) = &options.optimization {
        command.arg(level);
    }
    match options.mode {
        OutputMode::Executable => {}
        OutputMode::Object => {
            command.arg("-c");
        }
        OutputMode::Assembly => {
            command.arg("-S");
        }
        OutputMode::SyntaxOnly => {
            command.arg("-fsyntax-only");
        }
    }
    command.arg(&source_path);
    if let Some(output) = output {
        command.arg("-o").arg(output);
    }
    if options.verbose {
        eprintln!(
            "zenvik: invoking {} for {} input file(s)",
            Path::new(&compiler).display(),
            options.inputs.len()
        );
        eprintln!(
            "zenvik: mode {:?}",
            match options.mode {
                OutputMode::Executable => "executable",
                OutputMode::Object => "object",
                OutputMode::Assembly => "assembly",
                OutputMode::SyntaxOnly => "syntax-only",
            }
        );
        eprintln!(
            "zenvik: output {}",
            output.map_or_else(|| "<none>".into(), |path| path.display().to_string())
        );
    }
    let result = command.status().map_err(|error| {
        format!(
            "cannot run C++ backend `{}`: {error}",
            Path::new(&compiler).display()
        )
    });
    let cleanup = fs::remove_file(&source_path);
    if let Err(error) = cleanup
        && error.kind() != std::io::ErrorKind::NotFound
    {
        eprintln!(
            "zenvik: warning: cannot remove temporary file {}: {error}",
            source_path.display()
        );
    }
    let status = result?;
    Ok(status.code().unwrap_or(1))
}

fn print_help() {
    println!(
        "Zenvik {VERSION} - brace-delimited compiled language\n\
Usage: zenvik [options] file.zenvik [file.zenvik ...]\n\
\n\
Options:\n\
  -o PATH          Write output to PATH\n\
  -L DIR           Add a directory to the library search path\n\
  -c               Compile to a native object file\n\
  -S               Compile to assembly\n\
  -fsyntax-only    Check syntax and semantics without generating output\n\
  -O0|-O1|-O2|-O3  Select the native backend optimization level\n\
  -v               Print compilation details\n\
  --help           Show this help\n\
  --version        Show compiler version\n\
\n\
    Use `import library_name;` for libraries in libraries/ or `-L` paths.\n\
    Use `import \"relative/path.zenvik\";` for project source files.\n\
  Set ZENVIK_CXX to select the C++20 compiler used by the native backend."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_name_uses_input_stem() {
        assert_eq!(
            default_output(
                Path::new("examples/hello world.zenvik"),
                OutputMode::Executable
            ),
            PathBuf::from("examples/hello world")
        );
        assert_eq!(
            default_output(Path::new("hello.zenvik"), OutputMode::Object),
            PathBuf::from("hello.o")
        );
    }

    #[test]
    fn rejects_conflicting_output_modes() {
        let args = ["-c", "-S", "sample.zenvik"]
            .into_iter()
            .map(OsString::from)
            .collect();
        assert!(parse_arguments(args).is_err());
    }

    #[test]
    fn parses_compound_assignment_tokens() {
        let tokens = lexer::lex("count += 1\n", "test.zenvik").unwrap();
        assert!(matches!(
            &tokens[1].kind,
            lexer::TokenKind::Operator(operator) if operator == "+="
        ));
    }
}
