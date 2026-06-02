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

            let input = if input == "/multi" {
                read_multiline()?
            } else {
                input
            };

            if input.trim().is_empty() {
                continue;
            }

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
    println!("multi-line input; finish with a single '.' line");

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
