use chrono::Utc;
use slug::slugify;
use std::fs;

const POST_TEMPLATE: &'static str = include_str!("../templates/blog.md");

pub fn new_post(title: &str) -> i32 {
    let now = Utc::now();
    let date_str = now.format("%Y-%m-%d").to_string();
    let filename = format!("{}-{}-en.md", date_str, slugify(title));

    let content = POST_TEMPLATE
        .replace("{{title}}", title)
        .replace("{{date}}", &date_str);

    let path = format!("./content/blog/{}", filename);

    if let Err(e) = fs::write(&path, content) {
        eprintln!("Unable to create entry, error: {}", e);
        return 1;
    };

    println!("New blog in:\n\t{}", path);
    0
}
