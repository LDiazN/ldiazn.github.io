use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::io::Write;

#[derive(Debug, Serialize, Deserialize)]
struct Projects {
    entries: Vec<ProjectEntry>,
    highlight: ProjectEntry,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProjectEntry {
    title: String,
    url: String,
    desc: String,
    img: String,
    img_alt: Option<String>,
    date: String,
    tags: Vec<String>,
}

macro_rules! input {
    ($x:expr) => {{
        print!($x);
        io::stdout().flush().unwrap();
        let mut buff = String::new();
        if let Err(e) = io::stdin().read_line(&mut buff) {
            eprintln!("Error reading input: {e}");
            return 1;
        };

        buff.trim().to_string()
    }};
}

pub fn new_project() -> i32 {
    // Read new entry metadata
    println!("");
    let title = input!("\tTitle: ");
    let url = input!("\tURL: ");
    let desc = input!("\tDescripton: ");
    let img = input!("\tImage: ");
    let img_alt = input!("\tAlternative image (optional): ");
    let mut tags = vec![];
    println!("\tTags:");
    loop {
        let next_tag = input!("\t\t");

        if next_tag.is_empty() {
            break;
        }
        tags.push(next_tag);
    }
    let date = Utc::now().format("%b. %Y").to_string();

    let new_entry = ProjectEntry {
        title,
        url,
        desc,
        img,
        img_alt: if img_alt.is_empty() {
            None
        } else {
            Some(img_alt)
        },
        tags,
        date,
    };

    update(new_entry);

    0
}

fn update(new_entry: ProjectEntry) -> i32 {
    let projects_file = "./data/projects.yml";
    let content_str = match fs::read_to_string(projects_file) {
        Err(e) => {
            eprintln!("Unable to read current content: {e}");
            return 1;
        }
        Ok(c) => c,
    };

    let mut content: Projects = match serde_yml::from_str(&content_str) {
        Err(e) => {
            eprintln!("Unable to parse file content: {e}");
            return 1;
        }
        Ok(p) => p,
    };

    content.entries.push(new_entry);

    let new_content_str = match serde_yml::to_string(&content) {
        Err(e) => {
            eprintln!("Unable to serialize new version: {e}");
            return 1;
        }
        Ok(s) => s,
    };

    if let Err(e) = fs::write(projects_file, new_content_str) {
        eprintln!("Unable to write new content to fille: {e}");
        return 1;
    };

    0
}
