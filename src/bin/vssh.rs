/*
 * vssh.rs
 * 
 * Created on 10th of March 2026  
 * Author: Win (Thanawin) Pattanaphol
 * 
*/

use std::{env, io::{Write, stdin, stdout}, path::Path, process::Command};

fn main()
{
    loop 
    {
        // Displays the current path    
        let current_path = env::current_dir().unwrap();
        print!("{}$ ", current_path.display());
        stdout().flush();

        let mut input = String::new();
        stdin().read_line(&mut input).unwrap();

        // Removes the newline character
        let command = input.trim();
        
        // Tokenizing to get the args
        let mut tokens = input.trim().split_whitespace();
        let cmd = tokens.next().unwrap();
        let args = tokens;
        
        // Match cmds
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
            cmd =>
            {
                // Creating child processes from the command
                let mut child = Command::new(cmd)
                    .args(args)
                    .spawn()
                    .unwrap();

                // Wait for command to finish
                child.wait();
            }
        }   
    }
}