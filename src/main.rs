 use nix::sys::wait::waitpid;
use nix::unistd::{ForkResult, execvp, fork};
use std::ffi::CString;
use std::io::Write;
use std::{env, io};
use colored::Colorize;

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


fn run(words:&[&str]){
match unsafe { fork() } {
                    Ok(ForkResult::Child) => {
                        let c_words: Vec<CString> =
                            words.iter().map(|w| CString::new(*w).unwrap()).collect();
                        let _ = execvp(&c_words[0], &c_words);
                    }
                    Ok(ForkResult::Parent { child })=>{
                        waitpid(child, None).unwrap();
                    }
                    Err(e) => eprintln!("fork failed: {e}"),
                }
} 