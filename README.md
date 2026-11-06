# Tide

**Tide** is a statically typed, interpreted programming language built with Rust. It is designed around a straightforward syntax, familiar programming concepts, and a built-in standard library.

Tide runs through **Wave**, its interpreter and command-line tool.

> Tide is currently under active development.

## Features

- Statically declared variable types
- Integer and floating-point arithmetic
- Boolean logic and comparisons
- Functions with typed parameters
- Arrays with indexing and mutation
- Conditional statements (`if`, `else if`, `else`)
- `while`, C-style `for`, and `for-in` loops
- `break` and `continue`
- Function returns
- Module imports
- Built-in standard library
- File reading and writing
- Command-line argument access

## Getting Started

### Requirements

- Rust
- Cargo

### Installation

Clone the repository:

```bash
git clone https://github.com/LazyAshleyz/TideLang.git
cd TideLang
```

Install Wave:

```bash
cargo install --path .
```

Verify the installation:

```bash
wave --version
```

## Creating a Project

Wave provides an initialization command:

```bash
wave init
```

This generates the following project structure:

```text
MyProject/
├── Tide.toml
└── src/
    └── main.td
```

The generated `main.td` contains:

```tide
print("Hello, World!");
```

### Tide.toml

Every initialized project receives a `Tide.toml` configuration file.

Example:

```toml
[project]
name = "MyProject"
version = "1.0.0"
authors = ["Ashley", "Twlve"]
description = "A project written in Tide"

[language]
version = "1.0"
entry = "src/main.td"

[build]
mode = "interpreted"
output = "build/"

[compiler]
target = "native"
```

The configuration format is being established for future project and build tooling.

## Running Tide Programs

Run a Tide source file using Wave:

```bash
wave run src/main.td
```

You can also pass arguments to your Tide program:

```bash
wave run src/main.td hello world
```

Access them through `std.args()`.

## Language Syntax

### Data Types

Tide currently supports five declared types:

| Type | Description | Example |
|---|---|---|
| `int` | 64-bit integer | `42` |
| `ft` | Floating-point number | `3.14` |
| `bn` | Boolean | `true` |
| `str` | String | `"Hello"` |
| `array` | Array of values | `[1, 2, 3]` |

### Variables

Variables require an explicit type declaration.

```tide
int age = 20;
ft height = 1.85;
bn alive = true;
str name = "Ashley";

array numbers = [1, 2, 3];
```

Integer values can be widened to floating-point values:

```tide
ft number = 5;
```

### Operators

**Arithmetic**

| Operator | Operation |
|---|---|
| `+` | Addition |
| `-` | Subtraction |
| `*` | Multiplication |
| `/` | Division |
| `%` | Modulo |

**Assignment**

| Operator | Operation |
|---|---|
| `=` | Assignment |
| `+=` | Addition assignment |
| `-=` | Subtraction assignment |
| `*=` | Multiplication assignment |
| `/=` | Division assignment |
| `%=` | Modulo assignment |

**Comparison**

`==`, `!=`, `<`, `>`, `<=`, `>=`

**Logical**

`and`, `or`, `not`

### Functions

Functions are declared using `fn`.

```tide
fn add(int a, int b) {
    int result = a + b;
    return result;
}

int answer = add(10, 20);
print(answer);
```

Functions support typed parameters and return values.

Functions without an explicit return produce a `null` runtime value.

### Conditionals

```tide
int number = 10;

if (number > 10) {
    print("Greater than 10");
}
else if (number == 10) {
    print("Exactly 10");
}
else {
    print("Less than 10");
}
```

### Loops

**While loops**

```tide
int i = 0;

while (i < 5) {
    print(i);
    i += 1;
}
```

**C-style for loops**

```tide
for (int i = 0; i < 10; i += 1) {
    print(i);
}
```

**For-in loops**

```tide
array numbers = [1, 2, 3, 4, 5];

for (int number in numbers) {
    print(number);
}
```

Tide also supports `break` and `continue`.

### Arrays

Arrays support indexing, assignment, and mutation.

```tide
array numbers = [10, 20, 30];

print(numbers[0]);

numbers[1] = 50;
numbers[2] += 10;
```

Arrays can also be modified using the standard library.

```tide
import std;

array numbers = [1, 2, 3];

std.push(numbers, 4);

int last = std.pop(numbers);

print(numbers);
```

Arrays use shared references, meaning assigning an array to another variable shares the same underlying array.

### Comments

Single-line comments use `//`.

```tide
// This is a Tide comment

int number = 10; // Inline comments also work
```

## Standard Library

Tide includes a built-in `std` module.

Import it using:

```tide
import std;
```

The `print()` function is always available and does not require importing `std`.

### Arrays and Strings

| Function | Description |
|---|---|
| `std.len(value)` | Returns the length of an array or string |
| `std.push(array, value)` | Adds an element to an array |
| `std.pop(array)` | Removes and returns the last element |
| `std.upper(str)` | Converts a string to uppercase |
| `std.lower(str)` | Converts a string to lowercase |
| `std.contains(value, item)` | Checks whether a string or array contains a value |
| `std.split(str, separator)` | Splits a string into an array |
| `std.join(array, separator)` | Joins array elements into a string |
| `std.range(end)` | Generates a range of integers |
| `std.range(start, end)` | Generates a range with a starting value |
| `std.range(start, end, step)` | Generates a range with a custom step |

### Mathematics

| Function | Description |
|---|---|
| `std.abs(number)` | Absolute value |
| `std.sqrt(number)` | Square root |
| `std.pow(base, exponent)` | Exponentiation |
| `std.floor(number)` | Rounds down |
| `std.ceil(number)` | Rounds up |
| `std.round(number)` | Rounds to the nearest integer |
| `std.min(a, b)` | Returns the smaller value |
| `std.max(a, b)` | Returns the larger value |

### Type Conversions

| Function | Description |
|---|---|
| `std.to_int(value)` | Converts a value to an integer |
| `std.to_float(value)` | Converts a value to a float |
| `std.to_str(value)` | Converts a value to a string |

### Input, Files and System

| Function | Description |
|---|---|
| `std.input()` | Reads user input |
| `std.input(prompt)` | Displays a prompt and reads input |
| `std.read_file(path)` | Reads a file |
| `std.write_file(path, text)` | Writes text to a file, replacing existing contents |
| `std.append_file(path, text)` | Appends text to a file |
| `std.file_exists(path)` | Checks whether a file exists |
| `std.lines(str)` | Splits a string into lines |
| `std.args()` | Returns command-line arguments |
| `std.exit(code)` | Exits the program |

All `std` functions require `import std;`.

## Wave CLI

Wave currently provides the following commands:

| Command | Description |
|---|---|
| `wave run <file.td>` | Executes a Tide program |
| `wave tokens <file.td>` | Displays lexer tokens |
| `wave ast <file.td>` | Displays the parsed AST |
| `wave init` | Initializes a Tide project |
| `wave --help` | Displays usage information |
| `wave --version` | Displays the Wave version |

## Project Structure

```text
TideLang/
├── src/
│   ├── ast/
│   │   └── mod.rs
│   ├── commands/
│   │   └── mod.rs
│   ├── interpreter/
│   │   └── mod.rs
│   ├── lexer/
│   │   └── mod.rs
│   ├── parser/
│   │   └── mod.rs
│   └── main.rs
├── Cargo.toml
└── Cargo.lock
```

## Development Status

Tide and Wave are actively being developed.

Currently, Tide runs through an interpreter implemented in Rust. Compilation support, additional tooling, and further language features are planned for future development.

The language and its toolchain may change as development progresses.

## Authors

- Ashley
- Twlve

---

**Tide & Wave** — Building a language, one feature at a time.
