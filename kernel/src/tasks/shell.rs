use core::{arch::asm, cell::OnceCell};

use alloc::{collections::BTreeMap, string::String, vec, vec::Vec};
use riscv::asm;
use simple_shell::{Shell, ShellCommand};

use crate::{
    ksched, print, println,
    util::{self, Result, SBIConsole},
};

pub async fn shell_task() -> ksched::TaskResult {
    let mut shell = create_shell();
    shell.run_async().await;
    ksched::TaskResult::Exit
}

pub fn create_shell<'a>() -> Shell<'a> {
    let mut shell = Shell::new(SBIConsole::print, SBIConsole::read_byte);
    let mut commands = BTreeMap::new();

    commands.insert(
        "help",
        ShellCommand {
            help: "print this help message",
            func: |_, shell| {
                shell.print_help_screen();
                Ok(())
            },
            aliases: &["?", "h"],
        },
    );

    commands.insert(
        "shutdown",
        ShellCommand {
            help: "shutdown the system",
            func: |_, _| {
                println!("\nshutting down the system");
                util::shutdown();
            },
            aliases: &["sd", "exit", "quit"],
        },
    );

    commands.insert(
        "reboot",
        ShellCommand {
            help: "reboot the system",
            func: |_, _| {
                println!("\nrebooting the system");
                util::reboot();
            },
            aliases: &[],
        },
    );

    commands.insert(
        "clear",
        ShellCommand {
            help: "clear the screen",
            func: |_, shell| {
                shell.clear_screen();
                Ok(())
            },
            aliases: &["cls"],
        },
    );

    commands.insert(
        "echo",
        ShellCommand {
            help: "print the given arguments",
            func: |args, _| {
                for arg in args {
                    print!("{} ", arg);
                }
                println!();
                Ok(())
            },
            aliases: &[],
        },
    );

    commands.insert(
        "panic",
        ShellCommand {
            help: "panic the kernel",
            func: |_, _| panic!("panic command invoked"),
            aliases: &[],
        },
    );

    commands.insert(
        "exception",
        ShellCommand {
            help: "trigger an exception",
            func: |_, _| {
                unsafe { asm!("ebreak") };
                Ok(())
            },
            aliases: &[],
        },
    );

    commands.insert(
        "interrupt",
        ShellCommand {
            help: "cause a timer interrupt",
            func: |_, _| crate::timer::set_interrupt(100000),
            aliases: &[],
        },
    );

    commands.insert(
        "devicetree",
        ShellCommand {
            help: "print the device tree",
            func: |_, _| {
                crate::dtb::print_dtb();
                Ok(())
            },
            aliases: &["dt"],
        },
    );

    shell.with_commands(commands)
}
