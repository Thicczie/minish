use colored::Colorize;
use nix::libc::dup2;
use nix::sys::wait::waitpid;
use nix::unistd::{ForkResult, execvp, fork};
use nix::unistd::{pipe, read, write};
use std::ffi::CString;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::{env, io};

fn main() {
    println!("Welcome to minish>");
    loop {
        print!("{}", "$minish~".green());
        match env::current_dir() {
            Ok(dir) => print!("{}> ", dir.display()),
            Err(_) => print!("?> "),
        }
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        if input.trim().len() > 0 {
            let words: Vec<&str> = input.split_whitespace().collect();

            match words[0] {
                "exit" => {
                    print!("exiting...");
                    break;
                }

                "cd" => {
                    let dir = words.get(1).unwrap_or(&"/");
                    match env::set_current_dir(&dir) {
                        Ok(_) => println!("Successfully changed directory"),
                        Err(e) => eprintln!("Failed to change directory: {}", e),
                    }
                }

                _ => run(&words),
            }
        }
    }
}

fn run(words: &[&str]) {
    // detect pipe
    if let Some(idx) = words.iter().position(|w: &&str| *w == "|") {
        let left_words = &words[..idx];
        let right_words: &[&str] = &words[idx + 1..];
        if right_words.is_empty() {
            return;
        }
        let (read_fd, write_fd) = pipe().expect("Failed to create pipe");

        //first fork
        let pid1: nix::unistd::Pid = match unsafe { fork() } {
            Ok(ForkResult::Child) => {
                // dupe write_fd to stdout
                unsafe {
                    dup2(write_fd.as_raw_fd(), 1);
                    drop(write_fd);
                    drop(read_fd);
                };
                run_exec(left_words);
            }
            Ok(ForkResult::Parent { child }) => child,
            Err(e) => {
                eprintln!("fork failed: {e}");
                return;
            }
        };

        //second fork
        let pid2 = match unsafe { fork() } {
            Ok(ForkResult::Child) => {
                // dupe read_fd to stdin
                unsafe {
                    dup2(read_fd.as_raw_fd(), 0);
                    drop(write_fd);
                    drop(read_fd);
                };
                run_exec(right_words);
            }
            Ok(ForkResult::Parent { child }) => child,
            Err(e) => {
                eprintln!("fork failed: {e}");
                return;
            }
        };
        drop(write_fd);
        drop(read_fd);
        waitpid(pid1, None).unwrap();
        waitpid(pid2, None).unwrap();
    } else {
        match unsafe { fork() } {
            Ok(ForkResult::Child) => {
                run_exec(words);
            }
            Ok(ForkResult::Parent { child }) => {
                waitpid(child, None).unwrap();
            }
            Err(e) => eprintln!("fork failed: {e}"),
        }
    }
}

fn run_exec(words: &[&str]) -> ! {
    // convert &str to CString
    let c_words: Vec<CString> = words.iter().map(|w| CString::new(*w).unwrap()).collect();
    match execvp(&c_words[0], &c_words) {
        Ok(_) => {}
        Err(e) => eprintln!("command failed: {e}"),
    }
    std::process::exit(127);
}
