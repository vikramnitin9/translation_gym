To compile, first set RUSTUP_HOME

```
export RUSTUP_HOME=... # Your path here
```

Then build and install the slicer binaries
```
rustup component add rust-src rustc-dev llvm-tools-preview
export RUSTFLAGS="-L $RUSTUP_HOME/toolchains/nightly-2022-08-08-x86_64-unknown-linux-gnu/lib"
export LD_LIBRARY_PATH="$RUSTUP_HOME/toolchains/nightly-2022-08-08-x86_64-unknown-linux-gnu/lib"
cargo install --debug --locked --path . --force
```
