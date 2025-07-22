use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::SystemTime;
use filetime::FileTime;

mod lib;

fn main() {
    eprintln!("Starting program");
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <input_file> <output_file>", args[0]);
        std::process::exit(1);
    }
    let input_file = &args[1];
    let output_file = &args[2];

    eprintln!("Reading input file: {}", input_file);
    let description = fs::read_to_string(input_file)
        .unwrap_or_else(|_| panic!("Failed to read input file: {}", input_file));

    let output_path = Path::new(output_file);
    let draft_path = format!("{}.draft", output_file);
    let orig_path = format!("{}.orig", output_file);
    let rej_path = format!("{}.rej", output_file);

    let pid = std::process::id();
    let req_path_gen = format!("/tmp/llm-req-{}-gen.txt", pid);
    let resp_path_gen = format!("/tmp/llm-req-{}-gen-resp.txt", pid);
    let req_path_eval = format!("/tmp/llm-req-{}-eval.txt", pid);
    let resp_path_eval = format!("/tmp/llm-req-{}-eval-resp.txt", pid);

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
        let specimen = fs::read_to_string(output_file).unwrap_or_default();
        format!(
            "Please verify that the description below (enclosed into <result-description></result-description>) matches the specimen (enclosed into <result-specimen></result-specimen>) as much as possible. If it does - then simply output the content of the result-specimen verbatim. If you find that there are imperfections in how result-specimen fulfills its purpose described in result-description, then improve it and output the full result, with your improvements. Do not delimit the result with anything, output it verbatim.\n\n<result-description>\n{}\n</result-description>\n\n<result-specimen>\n{}\n</result-specimen>",
            description, specimen
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

    eprintln!("Running first cargo check");
    let compile_check = Command::new("cargo")
        .args(&["check", "--message-format", "json"])
        .output()
        .expect("Failed to execute cargo check");

    let stdout = String::from_utf8_lossy(&compile_check.stdout);
    let mut first_compile_errors = Vec::new();
    for line in stdout.lines() {
        if line.contains(output_file) {
            first_compile_errors.push(line.to_string());
        }
    }

    let original_content = if output_path.exists() {
        fs::read_to_string(output_file).unwrap_or_default()
    } else {
        String::new()
    };

    if output_path.exists() {
        eprintln!("Renaming original file to: {}", orig_path);
        fs::rename(output_file, &orig_path)
            .unwrap_or_else(|_| panic!("Failed to rename original file"));
    }

    eprintln!("Running second cargo check");
    let second_compile_check = Command::new("cargo")
        .args(&["check", "--message-format", "json"])
        .output()
        .expect("Failed to execute second cargo check");

    let second_stderr = String::from_utf8_lossy(&second_compile_check.stderr);
    let mut second_compile_errors = Vec::new();
    for line in second_stderr.lines() {
        if line.contains(output_file) {
            second_compile_errors.push(line.to_string());
        }
    }

    let eval_prompt = format!(
        "Please CAREFULLY evaluate the below description (enclosed into <result-description></result-description>), and two outputs corresponding to this description, first one enclosed into \"<first-result></first-result>\" and the second enclosed into \"<second-result></second-result>\", with compile errors of first result included into \"<first-compile-errors></first-compile-errors>\" and second compile errors as \"<second-compile-errors></second-compile-errors>\", and evaluate which of the two is more precise and correct in implementing the description - and also which of them compiles! Then, if the first result is better, output the phrase 'First result is better.', if the second result is better, output the phrase 'The second implementation is better.'. Output only one of the two phrases, and nothing else\n\n<result-description>\n{}\n</result-description>\n\n<first-result>\n{}</first-result>\n\n<second-result>\n{}</second-result>\n\n<first-compile-errors>\n{}</first-compile-errors>\n\n<second-compile-errors>\n{}</second-compile-errors>",
        description, original_content, response, first_compile_errors.join("\n"), second_compile_errors.join("\n")
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
        if first_compile_errors.is_empty() {
            eprintln!("No compile errors, restoring original");
            if Path::new(&orig_path).exists() {
                fs::rename(&orig_path, output_file)
                    .unwrap_or_else(|_| panic!("Failed to restore original file"));
                let now = SystemTime::now();
                filetime::set_file_mtime(output_file, FileTime::from_system_time(now))
                    .expect("Failed to update mtime");
            }
            if Path::new(&draft_path).exists() {
                fs::rename(&draft_path, &rej_path)
                    .unwrap_or_else(|_| panic!("Failed to rename rejected draft"));
            }
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

    eprintln!("Program completed successfully");
}
