use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::SystemTime;
use filetime::FileTime;

mod lib;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <input_file> <output_file>", args[0]);
        std::process::exit(1);
    }
    eprintln!("Starting program {}", args[0]);
    let input_file = &args[1];
    let output_file = &args[2];

    eprintln!("Checking output file status with git");
    if Path::new(output_file).exists() {
        let git_status = Command::new("git")
            .args(&["status", "--porcelain", output_file])
            .output()
            .expect("Failed to execute git status");
        
        let output = String::from_utf8_lossy(&git_status.stdout);
        if !output.trim().is_empty() {
            eprintln!("Error: Output file has uncommitted changes");
            std::process::exit(1);
        }
    }

    eprintln!("Reading input file: {}", input_file);
    let description = lib::preprocess::preprocess(input_file);

    let output_path = Path::new(output_file);
    let draft_path = format!("{}.draft", output_file);
    let rej_path = format!("{}.rej", output_file);

    let pid = std::process::id();
    let req_path_gen = format!("/tmp/llm-req-{}-gen.txt", pid);
    let resp_path_gen = format!("/tmp/llm-req-{}-gen-resp.txt", pid);
    let req_path_eval = format!("/tmp/llm-req-{}-eval.txt", pid);
    let resp_path_eval = format!("/tmp/llm-req-{}-eval-resp.txt", pid);

    let original_content = if output_path.exists() {
        fs::read_to_string(output_file).unwrap_or_default()
    } else {
        String::new()
    };

    let first_compiler_errors = if output_path.exists() {
        run_cargo_check(output_file)
    } else {
        eprintln!("No cargo check");
        Vec::new()
    };

    let prompt = if !output_path.exists()
        || fs::metadata(output_path)
            .map(|m| m.len() == 0)
            .unwrap_or(true)
    {
        eprintln!("Output file doesn't exist or is empty - using initial prompt");
        format!(
            "Please produce single output result, which would match the description below as well as you can:\n\n{}",
            description
        )
    } else {
        eprintln!("Output file exists - using verification prompt");
        format!(
            "Please verify that the description below (enclosed into <result-description></result-description>) matches the specimen (enclosed into <result-specimen></result-specimen>) as much as possible, taking into account the possible presence of compiler errors (enclosed into <compiler-errors></compiler-errors>. If it does - then simply output the content of the result-specimen verbatim. If you find that there are imperfections in how result-specimen fulfills its purpose described in result-description, then improve it and output the full result, with your improvements. Do not delimit the result with anything, output it verbatim.\n\n<result-description>\n{}\n</result-description>\n\n<result-specimen>\n{}\n</result-specimen>\n\n<compiler-errors>\n{}\n</compiler-errors>",
            description, original_content, first_compiler_errors.join("\n")
        )
    };

    eprintln!("Saving request to: {}", req_path_gen);
    fs::write(&req_path_gen, &prompt)
        .unwrap_or_else(|_| panic!("Failed to write request file: {}", req_path_gen));

    eprintln!("Calling Groq API");
    let groq = lib::groq::Groq::new();
    let response = groq.evaluate(&prompt);

    eprintln!("Saving response to: {}", resp_path_gen);
    fs::write(&resp_path_gen, &response)
        .unwrap_or_else(|_| panic!("Failed to write response file: {}", resp_path_gen));

    eprintln!("Writing draft to: {}", draft_path);
    fs::write(&draft_path, &response)
        .unwrap_or_else(|_| panic!("Failed to write draft file: {}", draft_path));

    let temp_path = format!("{}.tmp", output_file);
    fs::write(&temp_path, &response)
        .unwrap_or_else(|_| panic!("Failed to write temporary file"));
    
    let second_compiler_errors = run_cargo_check(&temp_path);

    let eval_prompt = format!(
        "Please CAREFULLY evaluate the below description (enclosed into <result-description></result-description>), and two outputs corresponding to this description, first one enclosed into \"<first-result></first-result>\" and the second enclosed into \"<second-result></second-result>\", with compile errors of first result included into \"<first-compile-errors></first-compile-errors>\" and second compile errors as \"<second-compile-errors></second-compile-errors>\", and evaluate which of the two is more precise and correct in implementing the description - and also which of them compiles! Then, if the first result is better, output the phrase 'First result is better.', if the second result is better, output the phrase 'The second implementation is better.'. Output only one of the two phrases, and nothing else\n\n<result-description>\n{}\n</result-description>\n\n<first-result>\n{}</first-result>\n\n<second-result>\n{}</second-result>\n\n<first-compile-errors>\n{}</first-compile-errors>\n\n<second-compile-errors>\n{}</second-compile-errors>",
        description, original_content, response, first_compiler_errors.join("\n"), second_compiler_errors.join("\n")
    );

    eprintln!("Saving evaluation request to: {}", req_path_eval);
    fs::write(&req_path_eval, &eval_prompt)
        .unwrap_or_else(|_| panic!("Failed to write evaluation request file"));

    eprintln!("Calling Groq API for evaluation");
    let groq_eval = lib::groq::Groq::new();
    let eval_response = groq_eval.evaluate(&eval_prompt);
    let trimmed = eval_response.trim();

    eprintln!("Saving evaluation response to: {}", resp_path_eval);
    fs::write(&resp_path_eval, &eval_response)
        .unwrap_or_else(|_| panic!("Failed to write evaluation response file"));

    eprintln!("Evaluation result: {}", trimmed);

    if trimmed == "First result is better." {
        eprintln!("First result is better");
        if first_compiler_errors.is_empty() {
            eprintln!("No compile errors, restoring original");
            if Path::new(&draft_path).exists() {
                fs::rename(&draft_path, &rej_path)
                    .unwrap_or_else(|_| panic!("Failed to rename rejected draft"));
            }
            let now = SystemTime::now();
            filetime::set_file_mtime(output_file, FileTime::from_system_time(now))
                .expect("Failed to update mtime");
        } else {
            eprintln!("First result better but has compile errors");
            if Path::new(&draft_path).exists() {
                fs::rename(&draft_path, &rej_path)
                    .unwrap_or_else(|_| panic!("Failed to rename rejected draft"));
            }
            std::process::exit(1);
        }
    } else if trimmed == "The second implementation is better." {
        eprintln!("Second implementation is better");
        fs::rename(&temp_path, output_file)
            .unwrap_or_else(|_| panic!("Failed to move temporary file to output file"));
        if Path::new(&draft_path).exists() {
            fs::remove_file(&draft_path)
                .unwrap_or_else(|_| panic!("Failed to remove draft file"));
        }
    } else {
        eprintln!("Unexpected evaluation response: {}", trimmed);
        std::process::exit(1);
    }

    eprintln!("Program completed successfully");
}

fn run_cargo_check(file_path: &str) -> Vec<String> {
    eprintln!("Running cargo check, focus on file {}", file_path);
    let output = Command::new("cargo")
        .args(&["check", "--message-format", "json"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute cargo check");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut errors = Vec::new();
    
    for line in stdout.lines() {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(rendered) = json.get("rendered").and_then(|v| v.as_str()) {
                if rendered.contains(file_path) && rendered.contains("err") {
                    eprintln!("COMPILER ERROR: {}", &rendered);
                    errors.push(rendered.to_string());
                }
            } else {
                eprintln!("STRANGE ERROR: {:?}", &json);
            }
        }
    }

    if errors.len() > 20 {
        errors.truncate(20);
    }

    errors
}
