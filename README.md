# coman

Contest manager — easily run and test your programming contest solutions.

[**Getting started**](https://github.com/j-tai/coman/wiki/Getting-started)

## How it works

* You put your source code anywhere in the `src` directory, or a subdirectory of `src`.
* You run `coman`. This will automatically find which source file you are working on and will compile and run it.
* You put tests in the `test` directory.
* You run `coman -T`. This will run each test and display the results.

Simple, right?

## Features

* Forget about writing Makefiles or regurgitating an obnoxiously long "gcc" command. *coman* takes the hassle away from compiling and running your code manually.
* It can automatically find which solution you are working on, and run that one. (Or you can specify the file on the command line.)
* It can test your solution with test cases that you provide.
* It can quickly open a debugger for you.
* It supports *any* programming language.

## Building and installing

To install this program, [install Rust](https://rustup.rs/) if you haven't already, clone the repository, and use cargo to install it:

```console
$ git clone https://github.com/j-tai/coman.git
$ cd coman
$ cargo install --path .
```

If you want to just build the program, you can use

```console
$ cargo build --release
```

for an optimized build (or omit `--release` for a debug build). Then, the binary will be in `target/release/coman` (or `target/debug/coman`).

## License

[GPLv3.](LICENSE)
