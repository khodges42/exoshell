use std::io::{self, Write};

use crate::app::{App, AppError};
use crate::formatting::render_assistant_output;

pub struct Repl {
    app: App,
}

impl Repl {
    pub fn new(app: App) -> Self {
        Self { app }
    }

    pub async fn run(mut self) -> Result<(), AppError> {
        println!("Exoshell Phase 1");
        println!(
            "Type /exit to quit. Type /multi to enter multi-line input, then finish with a single '.' line."
        );

        loop {
            print!("exo> ");
            io::stdout().flush().map_err(ReplError::Io)?;

            let Some(input) = read_input()? else {
                break;
            };

            let input = input.trim().to_string();
            if input.is_empty() {
                continue;
            }

            if input == "/exit" || input == "/quit" {
                break;
            }

            if input == "/add-output" {
                let output = read_multiline_with_prompt("paste command output")?;
                match self.app.add_command_output(output, None, None) {
                    Ok(message) => println!("{message}"),
                    Err(error) => eprintln!("context command failed: {error}"),
                }
                continue;
            }

            if input.starts_with("/context")
                || input.starts_with("/add-note ")
                || input.starts_with("/add-file ")
                || input.starts_with("/add-dir ")
            {
                match self.app.handle_command(&input) {
                    Ok(message) => println!("{message}"),
                    Err(error) => eprintln!("context command failed: {error}"),
                }
                continue;
            }

            let input = if input == "/multi" {
                read_multiline()?
            } else {
                input
            };

            if input.trim().is_empty() {
                continue;
            }

            println!("waiting for provider response...");
            match self.app.send(input).await {
                Ok(response) => println!("\n{}\n", render_assistant_output(&response)),
                Err(error) => eprintln!("request failed: {error}"),
            }
        }

        match self.app.save_transcript()? {
            Some(path) => println!("transcript: {}", path.display()),
            None => println!("transcript: disabled"),
        }

        Ok(())
    }
}

fn read_input() -> Result<Option<String>, ReplError> {
    let mut input = String::new();
    let bytes = io::stdin().read_line(&mut input).map_err(ReplError::Io)?;
    if bytes == 0 {
        return Ok(None);
    }

    Ok(Some(input))
}

fn read_multiline() -> Result<String, ReplError> {
    read_multiline_with_prompt("multi-line input")
}

fn read_multiline_with_prompt(prompt: &str) -> Result<String, ReplError> {
    println!("{prompt}; finish with a single '.' line");

    let mut lines = Vec::new();
    loop {
        print!("... ");
        io::stdout().flush().map_err(ReplError::Io)?;

        let Some(line) = read_input()? else {
            break;
        };

        let line = line.trim_end_matches(['\r', '\n']);
        if line == "." {
            break;
        }

        lines.push(line.to_string());
    }

    Ok(lines.join("\n"))
}

#[derive(Debug, thiserror::Error)]
pub enum ReplError {
    #[error("terminal I/O failed: {0}")]
    Io(std::io::Error),
}
