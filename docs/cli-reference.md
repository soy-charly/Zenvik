# CLI reference

```text
zenvik [options] file.zenvik [file.zenvik ...]
```

- `-o PATH`: output path. By default, the first input's extension is removed.
- `-c`: compile to an object file (`.o` by default).
- `-S`: emit native assembly (`.s` by default).
- `-fsyntax-only`: lex, parse, check, and validate generated C++ without writing a final artifact.
- `-O0`, `-O1`, `-O2`, `-O3`: pass the selected optimization level to the C++ backend.
- `-L DIR`, `--library-path DIR`: add a directory searched for named libraries.
- `-v`: show concise backend invocation details.
- `--help`, `--version`: print help or version.

`-c`, `-S`, and `-fsyntax-only` are mutually exclusive. `-o` is not valid with `-fsyntax-only`. Multiple source files are parsed and compiled together. A source file may import a project file with `import "relative/path.zenvik";`, or a library by name with `import math;`. Named imports search `-L` directories, directories in `ZENVIK_LIB_DIR`, the repository's `libraries/` directory, and `/usr/local/share/zenvik/libraries` in that order. On Windows or custom installations, configure a library directory with `-L` or `ZENVIK_LIB_DIR`. Imported paths are relative to the importing file, and each canonical file is included only once. Paths are passed as separate process arguments, so spaces are supported. Set `ZENVIK_CXX` to select the C++20 compiler; the default is `clang++`.

For example, `zenvik examples/modules/main.zenvik -o out/modules` compiles the main file and its relative import. To use the built-in repository library, write `import math;` and compile a source that calls `square(9)`.

The Zenvik front end uses exit code 2 for invalid input or source diagnostics, and propagates the native backend's exit status for backend errors.
