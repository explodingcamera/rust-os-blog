# simple-shell

[![crates.io](https://img.shields.io/crates/v/simple-shell.svg)](https://crates.io/crates/simple-shell)
[![docs.rs](https://docs.rs/simple-shell/badge.svg)](https://docs.rs/simple-shell)

A simple polling-based `no_std` shell for embedded systems.

## Usage

```rust,no_run
use simple_shell::{Shell, ShellCommand};

fn run_shell() {
    // Replace these with your console I/O functions.
    let (write, read) = (|text: &str| print!("{text}"), || None);

    let mut shell = Shell::new(write, read).with_commands([(
        "help",
        ShellCommand {
            help: "print this help message",
            func: |_, shell| {
                shell.print_help_screen();
                Ok(())
            },
            aliases: &["?", "h"],
        },
    )]);

    shell.run();
}
```
