# Suru

## A makefile replacement for the modern era

Suru is designed to be a modern replacement for Make. It implements modern
features such as relocatable build directories, automatic dependency scanning,
and more.

## Example

This is a sample of a minimal sufile. It contains the executable as well as the link instruction.
The compilation steps for c and cpp files are built-in to suru. All build commands must be done
via pattern matched recipes, which are separated from dependencies.

```Makefile

a: main.o lib/lib.o

% < %.o
    g++ -o $@ $^ -O3
```

## Alternatives

- Make
  - More available
  - Only capable of in-tree builds
- Ninja
  - Faster
  - Does not support pattern matching recipes
