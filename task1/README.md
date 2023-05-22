# Compiling the program

In case only the source code is available, the code can be built with Rust's standard tooling: cargo.
To get cargo either use a package available in the repositories of your operating system or get it
following the instructions for [installing Rust and it's toolchain available on rust-lang.org](https://www.rust-lang.org/tools/install).
After installing from the preferred source you can double check that cargo is installed by running:
```bash
rustup default stable #set "stable" as the release channel for the toolchain updates
rustup component add cargo #make sure cargo is included in the toolchain and download if not
```

To compile first make sure your working directory is the root of the repository or simply that in relation to this
README file your working directory can be expressed as `../`. Then run 
```sh
cargo build --bin task1
```

# Running the program
If you want to compile and run:
```sh
cargo run --bin task1 -- FILE_NAME NODE_NAME
```
To run compiled binary:
```sh
./task1 FILE_NAME NODE_NAME
```

_NODE_NAME_ is optional, _FILE_NAME_ is required. 

# Example input
The example input has been generated using another program built with `petgraph` and `serde_json` crates. Two files
can found under [`inputs/task1` directory](../inputs/task1). New files can be created manually modifying the `nodes`
and `edges` fields of the existing examples. `edges` uses indexes to reference nodes in the `nodes` section and
each array within should end with `null`.