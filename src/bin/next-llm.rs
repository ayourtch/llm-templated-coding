use std::{
    env,
    fs::{self, copy},
    path::Path,
    process::{Command, Stdio},
};

fn main() {
    // Get the prefix argument
    let arg = env::args().nth(1).expect("Usage: program <prefix>");
    let prefix = format!("llm-{}-", arg);

    let mut max_num = None;

    // Scan instruct/bin/ for .md files
    if let Ok(entries) = fs::read_dir("instruct/bin") {
        for entry in entries.filter_map(Result::ok) {
            let name = entry.file_name().into_string().unwrap_or_default();
            if let Some(num) = extract_number(&name, &prefix, ".md") {
                max_num = Some(max_num.map_or(num, |m: usize| m.max(num)));
            }
        }
    } else {
        eprintln!("Warning: could not read instruct/bin/");
    }

    // Scan src/bin/ for .rs files
    if let Ok(entries) = fs::read_dir("src/bin") {
        for entry in entries.filter_map(Result::ok) {
            let name = entry.file_name().into_string().unwrap_or_default();
            if let Some(num) = extract_number(&name, &prefix, ".rs") {
                max_num = Some(max_num.map_or(num, |m: usize| m.max(num)));
            }
        }
    } else {
        eprintln!("Warning: could not read src/bin/");
    }

    let current_num = match max_num {
        Some(n) => n,
        None => {
            eprintln!("No matching files found for prefix '{}'.", prefix);
            return;
        }
    };

    let next_num = current_num + 1;

    // Copy .md file
    let md_src = format!("instruct/bin/llm-{}-{}.md", arg, current_num);
    let md_dst = format!("instruct/bin/llm-{}-{}.md", arg, next_num);
    if !Path::new(&md_src).exists() {
        eprintln!("Error: source file {} does not exist", md_src);
        return;
    }
    copy(&md_src, &md_dst).expect("Failed to copy .md file");

    // Copy .rs file
    let rs_src = format!("src/bin/llm-{}-{}.rs", arg, current_num);
    let rs_dst = format!("src/bin/llm-{}-{}.rs", arg, next_num);
    if !Path::new(&rs_src).exists() {
        eprintln!("Error: source file {} does not exist", rs_src);
        return;
    }
    copy(&rs_src, &rs_dst).expect("Failed to copy .rs file");

    // Git add and commit .md file
    let status = Command::new("git")
        .args(&["add", &md_dst])
        .status()
        .expect("Failed to execute git add");
    if !status.success() {
        eprintln!("Failed to git add {}", md_dst);
        return;
    }

    let status = Command::new("git")
        .args(&["commit", "-m", &format!("first commit for llm-{}-{}", arg, next_num)])
        .status()
        .expect("Failed to execute git commit");
    if !status.success() {
        eprintln!("Failed to commit {}", md_dst);
        return;
    }

    // Git add and commit .rs file
    let status = Command::new("git")
        .args(&["add", &rs_dst])
        .status()
        .expect("Failed to execute git add");
    if !status.success() {
        eprintln!("Failed to git add {}", rs_dst);
        return;
    }

    let status = Command::new("git")
        .args(&["commit", "-m", &format!("first commit for llm-{}-{}", arg, next_num)])
        .status()
        .expect("Failed to execute git commit");
    if !status.success() {
        eprintln!("Failed to commit {}", rs_dst);
        return;
    }

    println!("Successfully created llm-{}-{} files and committed them", arg, next_num);
}

fn extract_number(name: &str, prefix: &str, suffix: &str) -> Option<usize> {
    name.strip_prefix(prefix)?
        .strip_suffix(suffix)?
        .parse()
        .ok()
}