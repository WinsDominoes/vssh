/*
 * vssh.rs
 * 
 * Created on 10th of March 2026  
 * Author: Win (Thanawin) Pattanaphol
 * 
*/

use std::{env, io::{Write, stdin, stdout}, process::Command};

fn main()
{
    loop 
    {
        // Displays the current path    
        let current_path = env::current_dir().unwrap();
        print!("{}$", current_path.display());
        stdout().flush();

        let mut input = String::new();
        stdin().read_line(&mut input).unwrap();

        // Removes the newline character
        let command = input.trim();

        // Creating child processes from the command
        let mut child = Command::new(command)
            .spawn()
            .unwrap();

        // Wait for command to finish
        child.wait();
    }
}