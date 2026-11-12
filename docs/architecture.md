# Compiler architecture

Zenvik's compiler is implemented in Rust with no Rust crate dependencies:

1. The lexer records file/line/column positions and tokenizes delimiters, literals, names, operators, newlines, and semicolons.
2. A recursive-descent/Pratt parser builds a typed Rust AST.
3. The CLI resolves top-level source imports relative to each importing file, loads each canonical source path once, and combines imported and command-line sources.
4. Semantic checks validate function declarations/calls and control-flow placement.
5. The backend emits C++20 source containing the program and a small embedded runtime.
6. The CLI invokes the selected host C++ compiler (default `clang++`) to link a native executable, object file, or assembly.

This C++ backend was selected over a custom machine-code backend or binding directly to a fast-moving LLVM API: it keeps the compiler front end small and maintainable while delegating target-specific code generation, optimization, and linking to a mature native toolchain. Generated C++ is a compiler backend representation, not a text substitution into Python or an interpreter. The final executable contains its runtime and runs without the Zenvik compiler. Building does require a C++20 compiler and standard library.

Values are dynamically typed. The embedded runtime represents them with a tagged variant; lists and dictionaries are reference-counted C++ containers. There is no static type checker, garbage collector, closure model, or stable cross-platform ABI yet. Calls between functions compiled in one invocation are emitted as native functions.

The backend compiles command-line inputs and imported files into a single translation unit. Imports are source inclusion without namespaces or separate compilation/linking; standalone object-file linking is not implemented.
