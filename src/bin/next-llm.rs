use std::{
    env,
    fs,
};

fn main() {
    let arg = env::args().nth(1).expect("Usage: find_max <prefix>");
    let prefix = format!("llm-{}-", arg);

    let mut max_num = None;

    // Scan instruct/bin/
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

    // Scan src/bin/
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

    match max_num {
        Some(n) => println!("{}", n),
        None => eprintln!("No matching files found for prefix '{}'.", prefix),
    }
}

fn extract_number(name: &str, prefix: &str, suffix: &str) -> Option<usize> {
    name.strip_prefix(prefix)?
        .strip_suffix(suffix)?
        .parse()
        .ok()
}