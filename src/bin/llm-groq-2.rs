use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::SystemTime;

mod lib;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <input_file> <output_file>", args[0]);
        std::process::exit(1);
    }
    let input_file = &args[1];
    let output_file = &args[2];

    let description = fs::read_to_string(input_file)
        .unwrap_or_else(|_| panic!("Failed to read input file: {}", input_file));

    let output_path = Path::new(output_file);
    let draft_path = format!("{}.draft", output_file);
    let orig_path = format!("{}.orig", output_file);
    let rej_path = format!("{}.rej", output_file);

    let pid = std::process::id();
    let req_path = format!("/tmp/llm-req-{}.txt", pid);

    let prompt = if !output_path.exists()
        || fs::metadata(output_path)
            .map(|m| m.len() == 0)
            .unwrap_or(true)
    {
        format!(
            "Please produce single output result, which would match the description below as well as you can:\n\n{}",
            description
        )
    } else {
        let specimen = fs::read_to_string(output_file).unwrap_or_default();
        format!(
            "Please verify that the description below (enclosed into <result-description></result-description>) matches the specimen (enclosed into <result-specimen></result-specimen>) as much as possible. If it does - then simply output the content of the result-specimen verbatim. If you find that there are imperfections in how result-specimen fulfills its purpose described in result-description, then improve it and output the full result, with your improvements. Do not delimit the result with anything, output it verbatim.\n\n<result-description>\n{}\n</result-description>\n\n<result-specimen>\n{}\n</result-specimen>",
            description, specimen
        )
    };

    fs::write(&req_path, &prompt)
        .unwrap_or_else(|_| panic!("Failed to write request file: {}", req_path));

    let groq = lib::groq::Groq::new();
    let response = groq.evaluate(&prompt);

    fs::write(&draft_path, &response)
        .unwrap_or_else(|_| panic!("Failed to write draft file: {}", draft_path));

    let compile_check = Command::new("cargo")
        .args(&["check", "--message-format", "json"])
        .output()
        .expect("Failed to execute cargo check");

    let stderr = String::from_utf8_lossy(&compile_check.stderr);
    let mut compile_errors = Vec::new();
    for line in stderr.lines() {
        if line.contains(output_file) {
            compile_errors.push(line.to_string());
        }
    }

    let original_content = if output_path.exists() {
        fs::read_to_string(output_file).unwrap_or_default()
    } else {
        String::new()
    };

    if output_path.exists() {
        fs::rename(output_file, &orig_path)
            .unwrap_or_else(|_| panic!("Failed to rename original file"));
    }

    let eval_prompt = format!(
        "Please CAREFULLY evaluate the below description (enclosed into <result-description></result-description>), and two outputs corresponding to this description, first one enclosed into \"<first-result></first-result>\" and the second enclosed into \"<second-result></second-result>\", with compile errors of second result included into \"<compile-errors></compile-errors>\", and evaluate which of the two is more precise and correct in implementing the description. Then, if the first result is better, output the phrase 'First result is better.', if the second result is better, output the phrase 'The second implementation is better.'. Output only one of the two phrases, and nothing else\n\n<result-description>\n{}\n</result-description>\n\n<first-result>\n{}\n</first-result>\n\n<second-result>\n{}\n</second-result>\n\n<compile-errors>\n{}\n</compile-errors>",
        description, original_content, response, compile_errors.join("\n")
    );

    let eval_response = groq.evaluate(&eval_prompt);
    let trimmed = eval_response.trim();

    if trimmed == "First result is better." {
        if compile_errors.is_empty() {
            if Path::new(&orig_path).exists() {
                fs::rename(&orig_path, output_file)
                    .unwrap_or_else(|_| panic!("Failed to restore original file"));
                let now = SystemTime::now();
                filetime::set_file_mtime(output_file, filetime::FileTime::from_system_time(now))
                    .expect("Failed to update mtime");
            }
            if Path::new(&draft_path).exists() {
                fs::rename(&draft_path, &rej_path)
                    .unwrap_or_else(|_| panic!("Failed to rename rejected draft"));
            }
        } else {
            if Path::new(&draft_path).exists() {
                fs::rename(&draft_path, &rej_path)
                    .unwrap_or_else(|_| panic!("Failed to rename rejected draft"));
            }
            eprintln!("First result better but has compile errors");
            std::process::exit(1);
        }
    } else if trimmed == "The second implementation is better." {
        fs::write(output_file, &response)
            .unwrap_or_else(|_| panic!("Failed to write output file"));
        if Path::new(&draft_path).exists() {
            fs::remove_file(&draft_path)
                .unwrap_or_else(|_| panic!("Failed to remove draft file"));
        }
    } else {
        eprintln!("Unexpected evaluation response: {}", trimmed);
        std::process::exit(1);
    }
}