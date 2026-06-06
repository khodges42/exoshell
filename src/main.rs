mod app;
mod commands;
mod config;
pub mod context;
mod formatting;
mod prompts;
mod providers;
mod repl;
mod shell;
mod transcripts;

use std::io::{IsTerminal, Read};

use crate::app::{App, CliOptions};
use crate::config::Config;
use crate::providers::openai_compatible::OpenAiCompatibleProvider;
use crate::repl::Repl;

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), app::AppError> {
    let options = CliOptions::parse(std::env::args().skip(1))?;

    if options.show_help {
        println!("{}", CliOptions::help());
        return Ok(());
    }

    let mut config = Config::load(options.config_path.as_deref())?;
    config.apply_cli_overrides(&options)?;
    let provider = OpenAiCompatibleProvider::from_config(&config)?;
    let mut app = App::new(config, Box::new(provider));

    for note in options.context_notes {
        println!("{}", app.add_note_context(note)?);
    }

    for path in options.context_files {
        println!("{}", app.add_file_context(path)?);
    }

    if !std::io::stdin().is_terminal() {
        let mut piped = String::new();
        std::io::stdin()
            .read_to_string(&mut piped)
            .map_err(|error| app::AppError::Repl(crate::repl::ReplError::Io(error)))?;
        if !piped.trim().is_empty() {
            println!("{}", app.add_stdin_context(piped)?);
        }
    }

    Repl::new(app).run().await
}
