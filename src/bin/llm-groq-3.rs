use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    process::{Command, Stdio},
};

mod lib;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <input_file> <output_file>", args[0]);
        std::process::exit(1);
    }
    let input_file = &args[1];
    let output_file = &args[2];

    let pid = std::process::id();

    let description = fs::read_to_string(input_file).expect("Failed to read input file");

    let prompt = if !Path::new(output_file).exists() || fs::metadata(output_file).unwrap().len() == 0 {
        format!(
            "Please produce single output result, which would match the description below as well as you can:\n\n{}",
            description
        )
    } else {
        let specimen = fs::read_to_string(output_file).expect("Failed to read output file");
        format!(
            "Please verify that the description below (enclosed into <result-description></result-description>) matches the specimen (enclosed into <result-specimen></result-specimen>) as much as possible. If it does - then simply output the content of the result-specimen verbatim. If you find that there are imperfections in how result-specimen fulfills its purpose described in result-description, then improve it and output the full result, with your improvements. Do not delimit the result with anything, output it verbatim.\n\n<result-description>\n{}\n</result-description>\n\n<result-specimen>\n{}\n</result-specimen>",
            description, specimen
        )
    };

    let req_file = format!("/tmp/llm-req-{}-gen.txt", pid);
    let mut file = File::create(&req_file).expect("Failed to create gen request file");
    file.write_all(prompt.as_bytes()).expect("Failed to write gen request");

    eprintln!("Sending generation request to LLM...");
    let groq = lib::groq::Groq::new();
    let response = groq.evaluate(&prompt);

    let resp_file = format!("/tmp/llm-req-{}-gen-resp.txt", pid);
    let mut file = File::create(&resp_file).expect("Failed to create gen response file");
    file.write_all(response.as_bytes()).expect("Failed to write gen response");

    eprintln!("Running cargo check on first result...");
    let first_errors = cargo_check(&response);

    let backup_file = format!("{}.orig", output_file);
    if Path::new(output_file).exists() {
        fs::rename(output_file, &backup_file).expect("Failed to rename output file");
    }

    let mut file = File::create(output_file).expect("Failed to create output file");
    file.write_all(response.as_bytes()).expect("Failed to write output");

    eprintln!("Running cargo check on second result...");
    let second_errors = cargo_check(&response);

    let eval_prompt = format!(
        "Please CAREFULLY evaluate the below description (enclosed into <result-description></result-description>), and two outputs corresponding to this description, first one enclosed into \"<first-result></first-result>\" and the second enclosed into \"<second-result></second-result>\", with compile errors of first result included into \"<first-compile-errors></first-compile-errors>\" and second compile errors as \"<second-compile-errors></second-compile-errors>\", and evaluate which of the two is more precise and correct in implementing the description - and also which of them compiles! Then, if the first result is better, output the phrase 'First result is better.', if the second result is better, output the phrase 'The second implementation is better.'. Output only one of the two phrases, and nothing else\n\n<result-description>\n{}\n</result-description>\n\n<first-result>\n{}\n</first-result>\n\n<first-compile-errors>\n{}\n</first-compile-errors>\n\n<second-result>\n{}\n</second-result>\n\n<second-compile-errors>\n{}\n</second-compile-errors>",
        description, response, first_errors, response, second_errors
    );

    let eval_req_file = format!("/tmp/llm-req-{}-eval.txt", pid);
    let mut file = File::create(&eval_req_file).expect("Failed to create eval request file");
    file.write_all(eval_prompt.as_bytes()).expect("Failed to write eval request");

    eprintln!("Sending evaluation request to LLM...");
    let eval_response = groq.evaluate(&eval_prompt);

    let eval_resp_file = format!("/tmp/llm-req-{}-eval-resp.txt", pid);
    let mut file = File::create(&eval_resp_file).expect("Failed to create eval response file");
    file.write_all(eval_response.as_bytes()).expect("Failed to write eval response");

    let eval_response = eval_response.trim();
    if eval_response == "The second implementation is better." {
        eprintln!("Using second implementation...");
    } else if eval_response == "First result is better." {
        eprintln!("Using first implementation...");
        if first_errors.is_empty() {
            fs::rename(&backup_file, output_file).expect("Failed to restore original file");
            let now = std::time::SystemTime::now();
            filetime::set_file_mtime(output_file, filetime::FileTime::from_system_time(now)).expect("Failed to update mtime");
        } else {
            let draft_file = format!("{}.rej", output_file);
            fs::rename(output_file, &draft_file).expect("Failed to rename to rej");
            fs::rename(&backup_file, output_file).expect("Failed to restore original file");
        }
    } else {
        eprintln!("Unexpected evaluation response: {}", eval_response);
        std::process::exit(1);
    }
}

use std::path::Path;

fn cargo_check(code: &str) -> String {
    let tmp_dir = format!("/tmp/cargo-check-{}", std::process::id());
    fs::create_dir_all(&tmp_dir).expect("Failed to create temp dir");
    fs::create_dir_all(format!("{}/src", tmp_dir)).expect("Failed to create src dir");

    let cargo_toml = r#"[package]
name = "temp"
version = "0.1.0"
edition = "2021"
"#;

    fs::write(format!("{}/Cargo.toml", tmp_dir), cargo_toml).expect("Failed to write Cargo.toml");
    fs::write(format!("{}/src/main.rs", tmp_dir), code).expect("Failed to write main.rs");

    let output = Command::new("cargo")
        .current_dir(&tmp_dir)
        .arg("check")
        .arg("--message-format=json")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to run cargo check");

    fs::remove_dir_all(&tmp_dir).expect("Failed to remove temp dir");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let mut errors = Vec::new();

    for line in stderr.lines() {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(reason) = json.get("reason").and_then(|v| v.as_str()) {
                if reason == "compiler-message" {
                    if let Some(message) = json.get("message") {
                        if let Some(rendered) = message.get("rendered").and_then(|v| v.as_str()) {
                            errors.push(rendered.to_string());
                        }
                    }
                }
            }
        }
    }

    errors.join("\n")
}