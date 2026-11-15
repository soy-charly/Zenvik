# Compiler architecture

Zenvik's compiler is implemented in Rust with no Rust crate dependencies:

1. The lexer records file/line/column positions and tokenizes delimiters, literals, names, operators, newlines, and semicolons.
2. A recursive-descent/Pratt parser builds a typed Rust AST.
3. The CLI resolves top-level source imports relative to each importing file, resolves named libraries from configured and repository library paths, loads each canonical source path once, and combines imported and command-line sources.
4. Semantic checks validate function declarations/calls and control-flow placement.
5. The backend emits C++20 source containing the program and a small embedded runtime.
6. The CLI invokes the selected host C++ compiler (default `clang++`) to link a native executable, object file, or assembly.

This C++ backend was selected over a custom machine-code backend or binding directly to a fast-moving LLVM API: it keeps the compiler front end small and maintainable while delegating target-specific code generation, optimization, and linking to a mature native toolchain. Generated C++ is a compiler backend representation, not a text substitution into Python or an interpreter. The final executable contains its runtime and runs without the Zenvik compiler. Building does require a C++20 compiler and standard library.

The semantic pass infers scalar types for unannotated assignments, checks subsequent assignments and typed function calls, and validates explicit C/C++-style scalar declarations. The embedded runtime still represents values with a tagged variant; typed integer declarations are range-checked and lists and dictionaries remain reference-counted C++ containers. `float` and `double` share the runtime's `double` representation, and `def` functions retain dynamically typed parameters and returns. There is no garbage collector, closure model, or stable cross-platform ABI yet. Calls between functions compiled in one invocation are emitted as native functions.

The backend compiles command-line inputs and imported files into a single translation unit. Named libraries are source files stored in `libraries/` and are bundled with the repository installer; `-L` and `ZENVIK_LIB_DIR` can add external library directories. Imports are source inclusion without namespaces or separate compilation/linking; standalone object-file linking is not implemented.
