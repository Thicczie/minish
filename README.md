# MiniSh

A basic Unix shell written in Rust, built to learn how processes, file descriptors and pipes work under the hood.

## Features

- **Built-in commands:** `cd` and `exit`, handled by the shell itself
- **External commands:** any program on your `PATH` (`ls`, `pwd`, `echo`, `grep`…)
- **Single pipes:** e.g. `echo Hello world | wc -w`

## How it works

The program is a loop that reads user input, splits it into words, and runs it.

### Built-ins

`exit` and `cd` are handled explicitly by the shell. `cd` uses `env::set_current_dir()` because each process has its own current directory: a child process can't change its parent's directory, so if `cd` ran as a separate process, the shell would stay where it was.

### Running one command

1. `fork()` the shell. This creates a new process that is a copy of the shell: same memory, same file descriptors, running from the same line.
2. In the child, call `execvp()`. This **replaces** the child's program with the command (searched for in the `PATH` directories). It doesn't create a new process; the child keeps its PID and file descriptors.
3. If `execvp()` returns, it failed (e.g. command not found), so the child exits with code `127`. On success, nothing after `execvp()` ever runs.
4. The parent (the shell) waits for the child with `waitpid()`, then shows the prompt again.

### Running a pipe

When the input contains `|`, it's split into a left command and a right command, and each runs in its own child process.

`nix::unistd::pipe()` creates a pipe, a kernel buffer with a **read end** and a **write end**, each given as a file descriptor.

The left command's output has to become the right command's input. Programs always write to stdout (fd 1) and read from stdin (fd 0), so each child rewires its own descriptors with `dup2()` before calling `execvp()`:

- **Left child:** `dup2` the pipe's write end onto stdout. Its output goes into the pipe.
- **Right child:** `dup2` the pipe's read end onto stdin. Its input comes from the pipe.

Neither program knows it's connected to a pipe; it just uses fd 0 and fd 1 as usual.

```rust
// first fork: left side of the pipe
let pid1: nix::unistd::Pid = match unsafe { fork() } {
    Ok(ForkResult::Child) => {
        // point stdout at the pipe's write end
        unsafe { dup2(write_fd.as_raw_fd(), 1) };
        // close the original pipe descriptors
        drop(write_fd);
        drop(read_fd);
        run_exec(left_words);
    }
    Ok(ForkResult::Parent { child }) => child,
    Err(e) => {
        eprintln!("fork failed: {e}");
        return;
    }
};
```

The right side is the same, except it points **stdin** at the pipe's **read end**.

### Why every process must close the pipe descriptors

Take `ls -l | wc -w`. `wc` keeps reading until it receives **EOF** (end-of-file), and the kernel only sends EOF when **every copy of the pipe's write end, in every process, is closed**.

`fork()` copies the whole file descriptor table, so after both forks, the shell and both children each hold a copy of both pipe ends. If any of them keeps a write end open, `wc` waits forever and the shell waits forever for `wc`: the shell hangs.

So:

1. **Each child** closes the original pipe descriptors after `dup2`. It only needs its rewired stdin or stdout.
2. **The shell** closes both pipe ends after both forks, since it doesn't read or write the pipe itself.
3. When `ls` finishes and exits, the kernel closes its stdout, the last write end. `wc` gets EOF, prints its result and exits.

### Why the shell waits only after both forks

The shell starts both children first, closes its pipe descriptors, and only then waits for both. A pipe's buffer has a limited size: if the left command writes more than fits and nothing is reading yet, it blocks until a reader takes data out. Waiting for the left command before starting the right one could therefore hang forever. With `yes | head -3`, for example, `yes` never finishes on its own, so `head` must already be running.
