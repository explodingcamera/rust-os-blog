#![no_std]
extern crate alloc;

use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};

mod constants;
mod reader;
mod writer;
use constants::*;
use reader::*;
use writer::*;

#[derive(Clone, Copy)]
pub struct ShellCommand<'a> {
    pub help: &'a str,
    pub func: fn(&[&str], &mut Shell<'a>) -> Result<(), &'a str>,
    pub aliases: &'a [&'a str],
}

pub struct Shell<'a> {
    pub history: Vec<String>,
    pub commands: BTreeMap<&'a str, ShellCommand<'a>>,
    pub command: String,
    pub cursor: usize,
    write: Writer,
    read: fn() -> Option<u8>,
}

impl<'a> Shell<'a> {
    pub fn new(write: fn(&str), read: fn() -> Option<u8>) -> Self {
        Self {
            history: Vec::new(),
            commands: BTreeMap::new(),
            command: String::new(),
            cursor: 0,
            write: Writer::new(write),
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
        (Reader::new(self.read)).await
    }

    pub fn with_commands(mut self, mut commands: BTreeMap<&'a str, ShellCommand<'a>>) -> Self {
        self.commands.append(&mut commands);
        self
    }

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
            CTRL_C => self.process_command("exit".to_string()),
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
            print!(self.write, "\x08"); // Move cursor left
            print!(self.write, "{}", &self.command[self.cursor..]); // Print the remaining text
            print!(self.write, " "); // Clear last character
            print!(self.write, "\x1b[{}D", self.command.len() - self.cursor + 1);
            // Move cursor to the correct position
        }
    }

    fn handle_enter(&mut self) {
        println!(self.write, "");
        self.process_command(self.command.clone());
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
        self._handle_escape(b);
    }

    fn handle_escape(&mut self) {
        if self.get_char() != CSI {
            return;
        }
        let b = self.get_char();
        self._handle_escape(b);
    }

    fn _handle_escape(&mut self, b: u8) {
        match b {
            CSI_UP => {}
            CSI_DOWN => {}
            CSI_RIGHT => {
                if self.cursor < self.command.len() {
                    print!(self.write, "\x1b[1C");
                    self.cursor += 1;
                }
            }
            CSI_LEFT if self.cursor > 0 => {
                print!(self.write, "\x1b[1D");
                self.cursor -= 1;
            }
            _ => {}
        }
    }

    fn process_command(&mut self, command: String) {
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

    pub fn print_prompt(&mut self) {
        print!(self.write, "> ");
    }

    pub fn clear_screen(&mut self) {
        print!(self.write, "\x1b[2J\x1b[1;1H");
    }
}
