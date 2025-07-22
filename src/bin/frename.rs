use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process,
};
use regex::Regex;

fn main() {
    let args: Vec<String> = env::args().collect();
    let dry_run = !args.iter().any(|x| x == "--do-rename");

    let positional: Vec<&str> = args.iter()
        .filter(|x| x.as_str() != "--do-rename")
        .map(|x| x.as_str())
        .collect();

    if positional.len() != 4 {
        eprintln!("Usage: {} <directory> <regex> <replacement> [--do-rename]", args[0]);
        process::exit(1);
    }

    let dir = Path::new(positional[1]);
    if !dir.is_dir() {
        eprintln!("Error: '{}' is not a directory", dir.display());
        process::exit(1);
    }

    let pattern = positional[2];
    let replacement = positional[3];

    // Detect simple extension-only replacement
    let simple = pattern.starts_with('.') && replacement.starts_with('.');
    let re = if simple {
        None
    } else {
        match Regex::new(pattern) {
            Ok(r) => Some(r),
            Err(e) => {
                eprintln!("Regex error: {}", e);
                process::exit(1);
            }
        }
    };

    let mut targets = Vec::new();
    visit_dir(dir, &mut targets).unwrap_or_else(|e| {
        eprintln!("Error walking directory: {}", e);
        process::exit(1);
    });

    let mut replacements = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for path in targets {
        let file_name = path.file_name().unwrap().to_string_lossy();
        let new_name = if simple {
            let stem = path.file_stem().unwrap().to_string_lossy();
            format!("{}{}", stem, replacement)
        } else {
            let re = re.as_ref().unwrap();
            if !re.is_match(&file_name) {
                continue;
            }
            re.replace(&file_name, replacement).into_owned()
        };

        if new_name == file_name {
            continue;
        }

        let new_path = path.parent().unwrap().join(&new_name);
        if seen.contains(&new_path) {
            eprintln!("Error: replacement name '{}' would clash for multiple files", new_name);
            process::exit(1);
        }
        seen.insert(new_path.clone());

        replacements.push((path, new_path));
    }

    if replacements.is_empty() {
        println!("No files to rename.");
        return;
    }

    if dry_run {
        println!("Dry-run mode. Would rename:");
        for (old, new) in &replacements {
            println!("  {} -> {}", old.display(), new.display());
        }
    } else {
        for (old, new) in replacements {
            if let Err(e) = fs::rename(&old, &new) {
                eprintln!("Failed to rename {} to {}: {}", old.display(), new.display(), e);
            } else {
                println!("Renamed {} -> {}", old.display(), new.display());
            }
        }
    }
}

fn visit_dir(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            visit_dir(&path, out)?;
        } else {
            out.push(path);
        }
    }
    Ok(())
}
