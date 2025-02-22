# Suru

## A makefile replacement for the modern era

Suru is designed to be a modern replacement for Make. It implements modern
features such as relocatable build directories, automatic dependency scanning,
and more. It is intended for small to medium sized projects where dependency management is not a concern.

## Example

This is a sample of a minimal sufile. It contains the executable as well as the link instruction.
The compilation steps for c and cpp files are built-in to suru. All build commands must be done
via pattern matched recipes, which are separated from dependencies.

```Makefile

a: main.o lib/lib.o

% < %.o
    g++ -o $@ $^ -O3
```

## How do I?

### `make all`

```sh
suru
```

By default suru builds all targets.

### `make clean`

```sh
rm -rd *
```

suru supports using build directories, so you can just build in a seperate directory and remove the build directory to clean up files.

### `make install`

suru does not natively support installing applications, although neither does make. `make install` simply runs a script that installs the software.

## Syntax

### Statements

In suru, there are three types of statements: variable declarations, tasks, and recipes. Variable declarations declare variables.

```makefile
CPPFLAGS = -D CONFIG_MACRO=1
```

Tasks specify a target to be build and their dependencies. Tasks with dependencies can have their dependent tasks deduced from recipes. Tasks do not specify how to build the target.

```makefile
a.exe: main.o
```

Recipes are like makefile pattern rules, and contain steps on how to build a target. They are not shell expressions, but use their own syntax. The `%` is replaced with the target name, while `*` matches anything. For example:

```makefile
%.exe: *.o
```

The above rule would match the `a.exe` example above, since main.o is matches the `*.o`.

```makefile
%.exe: %.o
```

This however would not match the `a.exe` example above, since it would only match `a.o`. `*` Rules in general can match any number of dependencies, while `%` can only match one.

### Expressions

Expressions in suru can either be a string literal, a variable, or a function. For example:

```makefile
LITERAL = foo bar
/* Same as LITERAL */
VAR = $(LITERAL)
/* equals to FOO BAR */
FUN = $(upper $(LITERAL))
```

Expressions can be found in variable declarations or recipe steps.

## Other notes

suru is not a shell invoker due to poor Rust support. This means shell expressions such as pipe or environment variables do not work. In order to invoke
shell expressions, a shell script can be used instead. See the [complex example](examples/complex/tasks.su) for a case where a shell file is invoked as a dependency.

## Alternatives

- Make
  - More mature
  - Only capable of in-tree builds
- Ninja
  - Faster
  - Does not support pattern matching recipes
