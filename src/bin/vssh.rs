/*
 * vssh.rs
 * 
 * Created on 10th of March 2026  
 * Author: Win (Thanawin) Pattanaphol
 * 
*/

use nix::sys::wait::{WaitPidFlag, WaitStatus, waitpid};
use nix::unistd::{ForkResult, close, dup2, execvp, fork, pipe};
use std::{env};
use std::ffi::CString;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::io::IntoRawFd;
use std::path::Path;

// File descriptors
const STDIN: i32 = 0;
const STDOUT: i32 = 1;

// Basic structure that holds parsed data
// from shell input
struct CmdLine
{
    cmds: Vec<String>,
    input_file: Option<String>,
    output_file: Option<String>,
    is_background: bool
}

/* Parsing */
fn parse_line(input: &str) -> CmdLine
{
    let mut line = input.trim().to_string();
    let mut is_background = false;

    // Check if background process
    if line.ends_with('&') 
    {
        is_background = true;
        line = line[..line.len() - 1].trim().to_string();
    }

    // Split | for pipeline
    let mut commands: Vec<String> = line
        .split('|')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // If command is NULL 
    if commands.is_empty()
    {
        return CmdLine 
        { 
            cmds: commands, 
            input_file: None, 
            output_file: None, 
            is_background: is_background 
        };
    }

    // If there is any > for output redirection
    let mut output_file = None;
    if let Some(last) = commands.last_mut()
    {
        if let Some(pos) = last.find('>')
        {
            let fname = last[pos + 1..].trim().to_string();
            // Woah! Removing > filename from the cmd
            *last = last[..pos].trim().to_string(); 
            
            if !fname.is_empty()
            {
                output_file = Some(fname)
            }
        }
    }

    // If there is any < in the command for input redirection
    let mut input_file: Option<String> = None;
    if let Some(first) = commands.first_mut()
    {
        if let Some(pos) = first.find('<')
        {
            let fname = first[pos + 1..].trim().to_string();
            *first = first[..pos].trim().to_string();

            if !fname.is_empty()
            {
                input_file = Some(fname);
            }
        }
    }

    CmdLine 
    { 
        cmds: commands, 
        input_file: input_file, 
        output_file: output_file, 
        is_background: is_background
    }
}

fn exec_pipeline(cmd_line: &CmdLine)
{
    let cmds = &cmd_line.cmds;
    let n = cmds.len();

    let mut prev_read: Option<i32> = None;

    for(i, cmd_str) in cmds.iter().enumerate()
    {
        let is_first = i == 0;
        let is_last = i == n -1;

        let pipe_fds = if !is_last
        {
            match pipe()
            {
                Ok((r, w)) => Some((r.into_raw_fd(), w.into_raw_fd())),
                Err(e) => 
                {
                    eprintln!("vssh: pipe: {}", e);
                    return;
                }
            }
        } 
        else 
        {
            None
        };

        // Forking processes now, interesting stuff!
        match unsafe { fork() }
        {
            // Parent process
            Ok(ForkResult::Parent { child }) => 
            {
                // Child inherits open fds -> parent closes it
                if let Some(r) = prev_read
                {
                    let _ = close(r);
                }

                // Save read-end of current pipe
                prev_read = pipe_fds.map(|(r, w)|
                {
                    let _ = close(w);
                    r
                });

                if is_last
                {
                    if cmd_line.is_background
                    {
                        println!("Starting background process [PID: {}]", child);
                    } 
                    else
                    {
                        let _ = waitpid(child, None);
                    }
                }
            }
            
            // Child process
            Ok(ForkResult::Child) =>
            {
                if is_first
                {
                    if let Some(ref path) = cmd_line.input_file
                    {
                        match File::open(path)
                        {
                            Ok(f) => 
                            {
                                let fd = f.into_raw_fd();
                                // Causes bugs (stdin does not redirect to the file)
                                
                                // let _ = unsafe { dup2(fd, STDIN) };
                                // let _ = close(fd);

                                if let Err(e) = dup2(fd, STDIN)
                                {
                                    eprintln!("vssh: dup2 stdin: {}", e);
                                    std::process::exit(1);
                                }

                                if fd != STDIN
                                {
                                    let _ = close(fd);
                                }
                            }
                            Err(e) => 
                            {
                                eprintln!("vssh: {}: {}", path, e);
                                std::process::exit(1);
                            }
                        }
                    }
                }
                else if let Some(r) = prev_read
                {
                    let _ = dup2(r, STDIN);
                    let _ = close(r);
                }

                if is_last
                {
                    if let Some(ref path) = cmd_line.output_file
                    {
                        match OpenOptions::new()
                            .write(true)
                            .create(true)
                            .truncate(true)
                            .open(path)
                        {
                            Ok(f) => 
                            {
                                let fd = f.into_raw_fd();
                                let _ = dup2(fd, STDOUT);
                                let _ = close(fd);
                            }
                            Err(e) => 
                            {
                                eprintln!("vssh: {}: {}", path, e);
                                std::process::exit(1);
                            }
                        }
                    }
                } else if let Some((r, w)) = pipe_fds
                {
                    let _ = close(r);
                    let _ = dup2(w, STDOUT);
                    let _ = close(w);
                }

                run_exec(cmd_str);
            }

            Err(e) => eprintln!("vssh: fork: {}", e),
        }
    }
}

// Converts command strfing into a C-compatible string to call execvp
fn run_exec(cmd_str: &str)
{
    let args: Vec<CString> = cmd_str
        .split_whitespace()
        .filter_map(|s| CString::new(s).ok())
        .collect();

    if args.is_empty()
    {
        std::process::exit(0);
    }

    if let Err(e) = execvp(&args[0], &args)
    {
        eprintln!("vssh: {}: {}", cmd_str.split_whitespace().next().unwrap_or(""), e);
    }

    std::process::exit(1);
}

fn main()
{
    loop 
    {
        // Check if there are any zombie background children
        loop
        {
            match waitpid(nix::unistd::Pid::from_raw(-1), Some(WaitPidFlag::WNOHANG))
            {
                Ok(WaitStatus::StillAlive) | Err(_) => break,
                Ok(_) => continue,
            }
        }

        // Displays the current path    
        let current_path = env::current_dir().unwrap();
        print!("{}$ ", current_path.display());
        let _ = io::stdout().flush();

        // Reading the input
        let mut input = String::new();
        // Clean exit if CTRL+D / EOF
        if io::stdin().read_line(&mut input).unwrap_or(0) == 0 
        {
            println!("\nexit");
            break;
        };

        let trimmed = input.trim();
        if trimmed.is_empty()
        {
            continue;
        }

        // let mut input_str = input.to_string();
        let mut tokens = trimmed.split_whitespace();
        let cmd = tokens.next().unwrap();
        let args = tokens; 

        match cmd
        {   
            // This is for if the cmd is `cd` which is special from other processes.
            "cd" => 
            {
                let new_dir = args.peekable().peek().map_or("/", |x| *x);
                let root = Path::new(new_dir);
                if let Err(e) = env::set_current_dir(&root)
                {
                    eprintln!("{}", e);
                }
            },
            "exit" => return,
            _ =>
            {
                let cmd_line = parse_line(trimmed);
                if !cmd_line.cmds.is_empty()
                {
                    exec_pipeline(&cmd_line);
                }
            },
        }  
    }
}