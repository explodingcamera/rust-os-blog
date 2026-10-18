//! A simple polling-based `no_std` shell for embedded systems.
//!
//! Pass [`Shell::new`] functions for writing text and reading bytes, then
//! register commands in [`Shell::commands`] or with [`Shell::with_commands`].
//! Use [`Shell::run`] for a blocking loop or [`Shell::run_async`] in an async
//! context.
//!
//! # Example
//!
//! ```no_run
//! use simple_shell::{Shell, ShellCommand};
//!
//! fn run_shell() {
//!     // Replace these with your console I/O functions.
//!     let (write, read) = (|text: &str| print!("{text}"), || None);
//!
//!     let mut shell = Shell::new(write, read).with_commands([(
//!         "help",
//!         ShellCommand {
//!             help: "print this help message",
//!             func: |_, shell| {
//!                 shell.print_help_screen();
//!                 Ok(())
//!             },
//!             aliases: &["?", "h"],
//!         },
//!     )]);
//!
//!     shell.run();
//! }
//! ```
#![no_std]
extern crate alloc;

use alloc::{collections::BTreeMap, string::String, vec::Vec};
use core::{future::poll_fn, task::Poll};

mod constants;
mod writer;
use constants::*;
use writer::*;

/// A command registered with a [`Shell`].
///
/// The handler receives whitespace-separated arguments, excluding the command
/// name. It can return an error message to print.
#[derive(Clone, Copy)]
pub struct ShellCommand<'a> {
    /// Description shown by [`Shell::print_help_screen`].
    pub help: &'a str,
    /// Handler for the command. Errors are printed with the command name.
    pub func: fn(&[&str], &mut Shell<'a>) -> Result<(), &'a str>,
    /// Alternative names for the command.
    pub aliases: &'a [&'a str],
}

/// An interactive command shell.
pub struct Shell<'a> {
    /// Submitted input lines, including empty lines.
    pub history: Vec<String>,
    /// Commands keyed by their primary names.
    pub commands: BTreeMap<&'a str, ShellCommand<'a>>,
    /// Current input line.
    pub command: String,
    /// Byte offset of the cursor within the current input line.
    pub cursor: usize,

    write: ShellWriter,
    read: fn() -> Option<u8>,
}

impl<'a> Shell<'a> {
    /// Creates a shell with output and input functions.
    ///
    /// `read` should return `None` when no byte is available.
    pub fn new(write: fn(&str), read: fn() -> Option<u8>) -> Self {
        Self {
            history: Vec::new(),
            commands: BTreeMap::new(),
            command: String::new(),
            cursor: 0,
            write: ShellWriter::new(write),
            read,
        }
    }

    fn get_char(&mut self) -> u8 {
        loop {
            if let Some(c) = (self.read)() {
                return c;
            }
        }
    }

    async fn get_char_async(&mut self) -> u8 {
        poll_fn(|cx| match (self.read)() {
            Some(c) => Poll::Ready(c),
            None => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        })
        .await
    }

    /// Adds commands, replacing existing commands with the same name.
    pub fn with_commands(
        mut self,
        commands: impl IntoIterator<Item = (&'a str, ShellCommand<'a>)>,
    ) -> Self {
        self.commands.extend(commands);
        self
    }

    /// Runs the shell in a blocking loop.
    pub fn run(&mut self) {
        self.print_prompt();

        loop {
            let c = self.get_char();
            match c {
                ESCAPE => self.handle_escape(),
                _ => self.match_char(c),
            }
        }
    }

    /// Runs the shell in an async context.
    ///
    /// If `read` returns `None`, the shell wakes itself to poll again.
    pub async fn run_async(&mut self) {
        self.print_prompt();

        loop {
            let c = self.get_char_async().await;
            match c {
                ESCAPE => self.handle_escape_async().await,
                _ => self.match_char(c),
            }
        }
    }

    fn match_char(&mut self, b: u8) {
        match b {
            CTRL_C => self.process_command("exit"),
            CTRL_L => self.handle_clear(),
            ENTER => self.handle_enter(),
            BACKSPACE => self.handle_backspace(),
            c if (32..=126).contains(&c) => {
                self.command.insert(self.cursor, c as char);
                self.cursor += 1;

                if self.cursor < self.command.len() {
                    // Print the remaining text
                    print!(self.write, "{}", &self.command[self.cursor - 1..]);
                    // Move cursor to the correct position
                    print!(self.write, "\x1b[{}D", self.command.len() - self.cursor);
                } else {
                    print!(self.write, "{}", c as char);
                }
            }
            _ => {}
        }
    }

    fn handle_clear(&mut self) {
        self.clear_screen();
        self.print_prompt();
        print!(self.write, "{}", self.command);
        self.cursor = self.command.len();
    }

    fn handle_backspace(&mut self) {
        if self.cursor > 0 {
            self.command.remove(self.cursor - 1);
            self.cursor -= 1;
            self.write.print(ASCII_BACKSPACE); // Move cursor left
            print!(self.write, "{}", &self.command[self.cursor..]); // Print the remaining text
            print!(self.write, " "); // Clear last character
            print!(self.write, "\x1b[{}D", self.command.len() - self.cursor + 1);
            // Move cursor to the correct position
        }
    }

    fn handle_enter(&mut self) {
        println!(self.write, "");
        let command = self.command.clone();
        self.process_command(&command);
        self.history.push(self.command.clone());
        self.command.clear();
        self.cursor = 0;
        self.print_prompt();
    }
    async fn handle_escape_async(&mut self) {
        if self.get_char_async().await != CSI {
            return;
        }
        let b = self.get_char_async().await;
        self.handle_csi_key(b);
    }

    fn handle_escape(&mut self) {
        if self.get_char() != CSI {
            return;
        }
        let b = self.get_char();
        self.handle_csi_key(b);
    }

    fn handle_csi_key(&mut self, b: u8) {
        match b {
            CSI_UP => {}
            CSI_DOWN => {}
            CSI_RIGHT => {
                if self.cursor < self.command.len() {
                    self.write.print(ANSI_CURSOR_RIGHT);
                    self.cursor += 1;
                }
            }
            CSI_LEFT if self.cursor > 0 => {
                self.write.print(ANSI_CURSOR_LEFT);
                self.cursor -= 1;
            }
            _ => {}
        }
    }

    fn process_command(&mut self, command: &str) {
        let mut args = command.split_whitespace();
        let command = args.next().unwrap_or("");
        let args = args.collect::<Vec<_>>();

        for (name, shell_command) in &self.commands {
            if shell_command.aliases.contains(&command) || name == &command {
                return (shell_command.func)(&args, self).unwrap_or_else(|err| {
                    println!(self.write, "{}: {}", command, err);
                });
            }
        }

        if command.is_empty() {
            return;
        }

        println!(self.write, "{}: command not found", command);
    }

    /// Prints the registered commands, their descriptions, and aliases.
    pub fn print_help_screen(&mut self) {
        println!(self.write, "available commands:");
        for (name, command) in &self.commands {
            print!(self.write, "  {:<12}{:<25}", name, command.help);
            if !command.aliases.is_empty() {
                print!(self.write, "    aliases: {}", command.aliases.join(", "));
            }
            println!(self.write, "");
        }
    }

    /// Prints the `> ` prompt.
    pub fn print_prompt(&mut self) {
        print!(self.write, "> ");
    }

    /// Sends ANSI escape sequences to clear the screen and move the cursor home.
    pub fn clear_screen(&mut self) {
        self.write.print(ANSI_CLEAR_SCREEN);
    }
}
