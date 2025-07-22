use std::fs;
use std::path::{Path, PathBuf};

pub fn preprocess(file_name: &str) -> String {
    preprocess_with_depth(file_name, 0)
}

fn preprocess_with_depth(file_name: &str, depth: usize) -> String {
    if depth >= 32 {
        return "TOO MUCH NESTED INCLUDES".to_string();
    }

    let path = Path::new(file_name);
    let contents = fs::read_to_string(&path).expect("Unable to read file");
    let mut output = String::new();
    let mut cursor = 0;

    while let Some(start) = contents[cursor..].find("{!") {
        let start = cursor + start;
        let after_start = start + 2;
        let Some(end) = contents[after_start..].find("!}") else {
            output.push_str(&contents[cursor..after_start]);
            cursor = after_start;
            continue;
        };
        let end = after_start + end;
        let include_path = &contents[after_start..end];

        let resolved_path = if Path::new(include_path).is_absolute() {
            include_path.to_string()
        } else {
            path.parent()
                .unwrap_or_else(|| Path::new("."))
                .join(include_path)
                .to_string_lossy()
                .into_owned()
        };

        let included = preprocess_with_depth(&resolved_path, depth + 1);
        output.push_str(&contents[cursor..start]);
        output.push_str(&included);
        cursor = end + 2;
    }

    output.push_str(&contents[cursor..]);
    if !output.ends_with('\n') {
        output.push('\n');
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{NamedTempFile, tempdir};
    use std::fs::{self, File};
    use std::io::Write;

    #[test]
    fn test_no_includes() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "hello world").unwrap();
        let result = preprocess(file.path().to_str().unwrap());
        assert_eq!(result, "hello world\n");
    }

    #[test]
    fn test_single_include() {
        let mut included = NamedTempFile::new().unwrap();
        writeln!(included, "included content").unwrap();

        let mut main = NamedTempFile::new().unwrap();
        writeln!(main, "before").unwrap();
        write!(main, "{{!{}!}}", included.path().to_str().unwrap()).unwrap();

        let result = preprocess(main.path().to_str().unwrap());
        assert_eq!(result, "before\nincluded content\n");
    }

    #[test]
    fn test_nested_includes() {
        let mut inner = NamedTempFile::new().unwrap();
        writeln!(inner, "inner").unwrap();

        let mut middle = NamedTempFile::new().unwrap();
        writeln!(middle, "middle before").unwrap();
        write!(middle, "{{!{}!}}", inner.path().to_str().unwrap()).unwrap();
        writeln!(middle, "middle after").unwrap();

        let mut root = NamedTempFile::new().unwrap();
        writeln!(root, "root start").unwrap();
        write!(root, "{{!{}!}}", middle.path().to_str().unwrap()).unwrap();
        writeln!(root, "root end").unwrap();

        let result = preprocess(root.path().to_str().unwrap());
        assert_eq!(
            result,
            "root start\nmiddle before\ninner\nmiddle after\nroot end\n"
        );
    }

    #[test]
    fn test_multiple_includes() {
        let mut first = NamedTempFile::new().unwrap();
        writeln!(first, "first").unwrap();
        let mut second = NamedTempFile::new().unwrap();
        writeln!(second, "second").unwrap();

        let mut main = NamedTempFile::new().unwrap();
        writeln!(main, "start").unwrap();
        write!(main, "{{!{}!}}", first.path().to_str().unwrap()).unwrap();
        writeln!(main, "middle").unwrap();
        write!(main, "{{!{}!}}", second.path().to_str().unwrap()).unwrap();

        let result = preprocess(main.path().to_str().unwrap());
        assert_eq!(result, "start\nfirst\nmiddle\nsecond\n");
    }

    #[test]
    fn test_no_newline_at_end() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "no newline").unwrap();
        let result = preprocess(file.path().to_str().unwrap());
        assert_eq!(result, "no newline\n");
    }

    #[test]
    fn test_include_without_newline() {
        let mut included = NamedTempFile::new().unwrap();
        write!(included, "content").unwrap();

        let mut main = NamedTempFile::new().unwrap();
        write!(main, "{{!{}!}}", included.path().to_str().unwrap()).unwrap();

        let result = preprocess(main.path().to_str().unwrap());
        assert_eq!(result, "content\n");
    }

    #[test]
    fn test_relative_path() {
        let dir = tempdir().unwrap();
        let included_path = dir.path().join("included.txt");
        fs::write(&included_path, "included\n").unwrap();

        let main_path = dir.path().join("main.txt");
        fs::write(&main_path, format!("{{!{}!}}", "included.txt")).unwrap();

        let result = preprocess(main_path.to_str().unwrap());
        assert_eq!(result, "included\n");
    }

    #[test]
    fn test_malformed_include() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "start {{! missing_end").unwrap();
        let result = preprocess(file.path().to_str().unwrap());
        assert_eq!(result, "start {! missing_end\n");
    }

    #[test]
    fn test_max_depth() {
        let dir = tempdir().unwrap();
        let mut paths = Vec::new();
        
        for i in 0..33 {
            let path = dir.path().join(format!("file_{}.txt", i));
            let next_path = dir.path().join(format!("file_{}.txt", i + 1));
            let content = format!("{{!{}!}}", next_path.to_str().unwrap());
            fs::write(&path, content).unwrap();
            paths.push(path);
        }

        let result = preprocess(paths[0].to_str().unwrap());
        assert!(result.contains("TOO MUCH NESTED INCLUDES"));
    }

    #[test]
    fn test_circular_include() {
        let dir = tempdir().unwrap();
        let a_path = dir.path().join("a.txt");
        let b_path = dir.path().join("b.txt");

        fs::write(&a_path, format!("{{!{}!}}", "b.txt")).unwrap();
        fs::write(&b_path, format!("{{!{}!}}", "a.txt")).unwrap();

        let result = preprocess(a_path.to_str().unwrap());
        assert!(result.len() > 0);
    }
}

/*
Cargo.toml:
[package]
name = "text_preprocessor"
version = "0.1.0"
edition = "2021"

[dependencies]
tempfile = "3.10"
*/