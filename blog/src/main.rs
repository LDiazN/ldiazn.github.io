use std::process::exit;

use blog::posts::new_post;
use blog::projects::new_project;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "blog", version, about = "Create more blog more faster!")]
struct Blog {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    New {
        #[command(subcommand)]
        item: Item,
    },
}

#[derive(Subcommand, Debug)]
enum Item {
    Post { title: String },
    Project,
}

fn main() {
    let cli = Blog::parse();
    match cli.command {
        Commands::New {
            item: Item::Post { title },
        } => exit(new_post(&title)),
        Commands::New {
            item: Item::Project { .. },
        } => exit(new_project()),
    };
}
