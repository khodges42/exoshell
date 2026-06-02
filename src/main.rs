mod app;
mod config;
mod formatting;
mod prompts;
mod providers;
mod repl;
mod shell;
mod transcripts;

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
    let app = App::new(config, Box::new(provider));

    Repl::new(app).run().await
}
