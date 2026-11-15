# Zenvik language specification (0.1)

Zenvik is Python-inspired, not Python-compatible. `.zenvik` is the conventional source extension. Blocks use `{` and `}`; indentation is insignificant. Every simple statement must end with `;`. Newlines and semicolons separate statements outside parentheses, brackets, and dictionary literals. Block headers and closing braces do not need a semicolon. `#` begins a line comment.

## Values and names

Values include `None`, booleans (`True`, `False`), signed integers, floating-point numbers, strings, lists, and dictionaries with stringified keys. Names use ASCII letters or `_` initially, followed by letters, digits, or `_`. Variables are introduced by assignment and live in the current function's environment. Functions have their own local environment; global-variable lookup and closures are not implemented.

Scalar types are inferred on first assignment and checked on later assignments. Numeric types may convert to other numeric types; incompatible assignments are compile-time errors. Lists and dictionaries remain dynamically represented collections. Explicit C/C++-style declarations require an initializer:

```zenvik
int age = 20;
double temperature = 18.5;
bool enabled = True;
char initial = "A";
string greeting = "hello";
```

Supported scalar types are `bool`; `char`, `signed char`, `unsigned char`, `wchar_t`, `char8_t`, `char16_t`, and `char32_t`; `short`, `int`, `long`, and `long long` (with signed/unsigned variants); `float`, `double`, and `long double`; the fixed-width integer names `int8_t` through `int64_t` and `uint8_t` through `uint64_t`; `intptr_t`, `uintptr_t`, `intmax_t`, `uintmax_t`, `size_t`, `ptrdiff_t`; and `string`. Use the type names directly without a `std::` prefix. `void` is only valid as a function return type. Integer declarations enforce the target C++ type's range. Character values use one-byte strings, and floating-point values use the runtime's `double` representation. Pointers, references, arrays, structs, classes, and user-defined types are not supported.

Strings use single or double quotes and support `\\`, `\"`, `\'`, `\n`, `\r`, `\t`, and `\0`. Triple-quoted and interpolated strings are not implemented.

## Statements

```zenvik
value = 2;
value += 3;
int limit = 10;
limit -= 1;

import math;

if value > 3 {
    print("large");
} elif value == 3 {
    print("three");
} else {
    print("small");
}

while value > 0 {
    value -= 1;
}

for item in range(3) {
    print(item);
}

def twice(number) {
    return number * 2;
}

int sum(int left, int right) {
    return left + right;
}
```

<<<<<<< HEAD
Supported statements: `import library_name;`, `import "relative/path.zenvik";`, expression statements, `=`, `+=`, `-=`, `*=`, `/=`, `%=`, `if`/`elif`/`else`, `while`, `for name in iterable`, function definitions, `return`, `break`, and `continue`. Simple statements (imports, expressions, assignments, `return`, `break`, and `continue`) require a trailing `;`; compound statements (blocks and their headers) do not. Quoted paths resolve relative to the importing file. Named libraries resolve as `<library-name>.zenvik` in directories supplied with `-L`, directories in `ZENVIK_LIB_DIR`, the repository's `libraries/` directory, or the installed library directory, in that order. All imported source joins the same compilation, and each canonical file is included once. Imports must be top-level. Functions are top-level, have fixed positional parameters, and may recurse. Exceptions, classes, keyword/default arguments, nested functions, and assignment to an indexed element are not supported.
=======
Supported statements: expression statements, inferred or typed assignments, `=`, `+=`, `-=`, `*=`, `/=`, `%=`, `if`/`elif`/`else`, `while`, `for name in iterable`, function definitions, `return`, `break`, and `continue`. C-style function definitions specify a return type and typed parameters (`int sum(int left, int right)`); `void` functions may use `return;`. The existing `def` syntax remains supported, with dynamically typed parameters and return values. Simple statements (expressions, assignments, `return`, `break`, and `continue`) require a trailing `;`; compound statements (blocks and their headers) do not. Functions are top-level, have fixed positional parameters, and may recurse. Imports, exceptions, classes, keyword/default arguments, nested functions, and assignment to an indexed element are not supported.
>>>>>>> cfc282f (added standard libraries)

## Expressions and precedence

From lowest to highest: `or`; `and`; comparisons (`==`, `!=`, `<`, `<=`, `>`, `>=`, `in`); `+`, `-`; `*`, `/`, `%`; `**`; unary `not`, `+`, `-`; calls and indexing. Exponentiation is right-associative and binds more tightly than unary signs. Parentheses group expressions. `and` and `or` short-circuit and return one of their operands.

Lists use `[item, ...]`; dictionaries use `{key: value, ...}`. Indexing and slicing (`value[index]`, `value[start:end:step]`) are supported for strings and lists. Iteration and `in` support strings, lists, and dictionaries; iterating a dictionary yields its keys. `range`, `len`, `print`, `input`, `str`, `int`, `float`, `bool`, and `type` are built-ins. String arguments to `int` and `float` must contain a complete numeric value (surrounding whitespace is allowed).

## Runtime and errors

The runtime is embedded in each generated native program, uses Boost.Multiprecision for arbitrary-precision integers, and uses C++ standard-library values with shared ownership for collection storage. Collection literals, concatenation, indexing, slicing, membership, conversions, and iteration are implemented. Imports are source inclusion, not separately compiled modules; list/dictionary mutation methods and a module system with namespaces are not implemented.

Lexical, parse, and semantic errors report the source path, line, and column. Runtime errors are printed to standard error and exit with status 1. Numeric and collection behavior is intentionally a small subset; do not rely on exact Python coercion, formatting, Unicode indexing, or overflow behavior.
