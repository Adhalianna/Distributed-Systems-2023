# Solved tasks

#### Assignmnet 1

```sh
cargo run --bin task1 inputs/task1/example1.json F
```
Described by README in the [task1](task1/README.md) directory.

#### Assignmnet 2

```sh
cargo run --bin task2 inputs/task2/example1.json
```
Described by README in the [task2](task2/README.md) directory.

#### Assignmnet 3

```sh
cargo run --bin task2 inputs/task3/example2.json
```
Described by README in the [task3](task3/README.md) directory.


---

# The execution environment / compilation tools

## Getting the tools

All the projects within this workspace (a [cargo workspace](https://doc.rust-lang.org/cargo/reference/workspaces.html)
to be specific) are written in the [Rust programming language](https://www.rust-lang.org/).
To be able to work with the projects a build tool called cargo is required.
It is a part of the official Rust toolchain and as such can be installed with
the language's compiler. One can get all of that using `rustup` which is a
toolchain manager for Rust that simplifies using multiple versions of the
compiler or tools on a single system. Unless one prefers using a package
manager of a used system, getting rustup and having it install the compiler,
the package manager, the standard library, etc, is the recommended way of
starting developement with rust. To get rustup follow [the instructions at the
rust-lang.org website](https://www.rust-lang.org/tools/install).

Once `rustup` is installed run the following to make sure that the most recent
stable version of Rust is used and the required tools are installed alongside:
```sh
rustup default stable
rustup component add cargo
```

## Running specific assignments

The [crates](https://doc.rust-lang.org/book/ch07-01-packages-and-crates.html) 
/ binaries related to a specific assignments are simply called `task1`, 
`task2`, etc. As such to instruct `cargo` to operate on a selected binary 
within the workspace one should rember to add for example the following flag:
`--bin task2`. 

To just compile task2:
```sh
cargo build --bin task2
```

To compile and run task2:
```sh
cargo run --bin task2
```

To run with cargo and pass commandline arguments to the program:
```sh
cargo run --bin task2 -- inputs/task2/example1.json
```
(Note the `--` that is followed by a space. It serves as a separator for 
arguments of cargo and the binary.)

A program that has been compiled can also be found in the `target` directory
that should get created at the root of the workspace. By default programs are
built with the `debug` profile so for example the path to a program built with
command `cargo build --bin task2` can be found under `target/debug/task2`.
Such program is a standalone binary and can be safely moved outside of the
workspace.

To compile in the release mode (faster, possibly smaller binary) run:
```sh
cargo build --bin task2 --release
```


# GitHub repository

[The project is hosted on GitHub.](https://github.com/Adhalianna/Distributed-Systems-2023)
It is convenient and the risk of plagiarism is rather unlikely considering that
most other students have not even ever heard of Rust. It is a language
considered to be difficult to start with so if a person with no prior 
experience with it was asked to refactor anything they would quite noticeably
struggle with it.