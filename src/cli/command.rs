use clap::{Parser, Subcommand};

const ABOUT: &str = r"
   __        _ __           ____
  / /_____ _(_) /________ _/ / /
 / __/ __ `/ / / ___/ __ `/ / /
/ /_/ /_/ / / / /__/ /_/ / / /
\__/\__,_/_/_/\___/\__,_/_/_/";

#[derive(Parser)]
#[command(name ="tc",author, version, about, long_about = Some(ABOUT))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Starts the GraphQL server on the configured port
    Start {
        /// Path for the configuration file
        file_path: String,
    },

    /// Validate a composition spec
    Check {
        /// Path for the configuration file
        file_path: String,

        /// N plus one queries
        #[arg(short, long)]
        n_plus_one_queries: bool,

        /// Display schema
        #[arg(short, long)]
        schema: bool,
    },
}
