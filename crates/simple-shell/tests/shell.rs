use std::{
    cell::RefCell,
    collections::{BTreeMap, VecDeque},
    future::Future,
    task::{Context, Poll, Waker},
};

use simple_shell::{Shell, ShellCommand};

thread_local! {
    static OUTPUT: RefCell<String> = RefCell::new(String::new());
    static INPUT: RefCell<VecDeque<u8>> = RefCell::new(VecDeque::new());
}

fn write(text: &str) {
    OUTPUT.with(|output| output.borrow_mut().push_str(text));
}

fn read() -> Option<u8> {
    INPUT.with(|input| input.borrow_mut().pop_front())
}

#[test]
fn dispatches_aliases_and_reports_errors() {
    INPUT.with(|input| {
        input
            .borrow_mut()
            .extend(b"h foo\runknown\r\x03".iter().copied())
    });

    let mut commands = BTreeMap::new();
    commands.insert(
        "help",
        ShellCommand {
            help: "show commands",
            func: |args, _| {
                assert_eq!(args, ["foo"]);
                Err("failed")
            },
            aliases: &["h"],
        },
    );

    let mut shell = Shell::new(write, read)
        .with_commands(commands)
        .with_commands([(
            "exit",
            ShellCommand {
                help: "exit the shell",
                func: |_, _| Err("called"),
                aliases: &[],
            },
        )]);
    let mut run = Box::pin(shell.run_async());
    assert!(matches!(
        run.as_mut().poll(&mut Context::from_waker(Waker::noop())),
        Poll::Pending
    ));
    drop(run);

    assert_eq!(shell.history, ["h foo", "unknown"]);
    OUTPUT.with(|output| {
        let output = output.borrow();
        assert!(output.contains("h: failed\n"));
        assert!(output.contains("unknown: command not found\n"));
        assert!(output.contains("exit: called\n"));
    });
}
