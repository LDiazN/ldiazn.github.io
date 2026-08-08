use std::{fs, process::exit};

use chrono::Utc;
use clap::{Parser, Subcommand};
use slug::slugify;

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
    Project { name: String },
}

const POST_TEMPLATE: &'static str = include_str!("../templates/blog.md");

fn new_post(title: &str) {
    let now = Utc::now();
    let date_str = now.format("%Y-%m-%d").to_string();
    let filename = format!("{}-{}-en.md", date_str, slugify(title));

    let content = POST_TEMPLATE
        .replace("{{title}}", title)
        .replace("{{date}}", &date_str);

    let path = format!("./content/blog/{}", filename);

    if let Err(e) = fs::write(&path, content) {
        eprintln!("Unable to create entry, error: {}", e);
        exit(1);
    };

    println!("New blog in:\n\t{}", path)
}

fn main() {
    let cli = Blog::parse();
    match cli.command {
        Commands::New {
            item: Item::Post { title },
        } => new_post(&title),
        Commands::New {
            item: Item::Project { .. },
        } => println!("Not yet implemented"),
    };
}
