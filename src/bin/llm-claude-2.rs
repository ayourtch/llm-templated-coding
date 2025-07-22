[dependencies]
reqwest = { version = "0.11", features = ["json", "blocking"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
filetime = "0.2"

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, exit};
use filetime::{FileTime, set_file_mtime};
use serde_json::{json, Value};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <input_file> <output_file>", args[0]);
        exit(1);
    }

    let input_file = &args[1];
    let output_file = &args[2];
    let draft_file = format!("{}.draft", output_file);
    let reject_file = format!("{}.rej", output_file);

    let api_key = env::var("ANTHROPIC_API_KEY").unwrap_or_else(|_| {
        eprintln!("Error: ANTHROPIC_API_KEY environment variable not set");
        exit(1);
    });

    let input_content = fs::read_to_string(input_file).unwrap_or_else(|e| {
        eprintln!("Error reading input file {}: {}", input_file, e);
        exit(1);
    });

    let output_exists = Path::new(output_file).exists();
    let output_empty = if output_exists {
        fs::metadata(output_file).map(|m| m.len() == 0).unwrap_or(true)
    } else {
        true
    };

    let prompt = if !output_exists || output_empty {
        format!("Please produce single output result, which would match the description below as well as you can:\n\n{}", input_content)
    } else {
        let output_content = fs::read_to_string(output_file).unwrap_or_else(|e| {
            eprintln!("Error reading output file {}: {}", output_file, e);
            exit(1);
        });
        format!("Please verify that the description below (enclosed into <result-description></result-description>) matches the specimen (enclosed into <result-specimen></result-specimen>) as much as possible. If it does - then simply output the content of the result-specimen verbatim. If you find that there are imperfections in how result-specimen fulfills its purpose described in result-description, then improve it and output the full result, with your improvements, BUT without any side comments/observations. Do not delimit the result with anything, output it verbatim.\n\n<result-description>\n{}\n</result-description>\n\n<result-specimen>\n{}\n</result-specimen>", input_content, output_content)
    };

    println!("Calling Claude Sonnet 4...");
    let llm_response = call_claude(&api_key, &prompt, 4096, 0.7);

    fs::write(&draft_file, &llm_response).unwrap_or_else(|e| {
        eprintln!("Error writing draft file {}: {}", draft_file, e);
        exit(1);
    });

    if !output_exists || output_empty {
        fs::write(output_file, &llm_response).unwrap_or_else(|e| {
            eprintln!("Error writing output file {}: {}", output_file, e);
            exit(1);
        });
        fs::remove_file(&draft_file).ok();
        println!("Initial output created successfully.");
        return;
    }

    let original_content = fs::read_to_string(output_file).unwrap();
    let evaluation_prompt = format!(
        "Please CAREFULLY evaluate the below description (enclosed into <result-description></result-description>), and two outputs corresponding to this description, first one enclosed into \"<first-result></first-result>\" and the second enclosed into \"<second-result></second-result>\", and evaluate which of the two is more precise and correct in implementing the description. Then, if the first result is better, output the phrase 'First result is better.', if the second result is better, output the phrase 'The second implementation is better.'. Output only one of the two phrases, and NOTHING else - not your thoughts, not analysis.\n\n<result-description>\n{}\n</result-description>\n\n<first-result>\n{}\n</first-result>\n\n<second-result>\n{}\n</second-result>",
        input_content, original_content, llm_response
    );

    println!("Evaluating results...");
    let evaluation_response = call_claude(&api_key, &evaluation_prompt, 100, 0.1).trim().to_string();

    if evaluation_response == "The second implementation is better." {
        fs::write(output_file, &llm_response).unwrap_or_else(|e| {
            eprintln!("Error writing output file {}: {}", output_file, e);
            exit(1);
        });
        fs::remove_file(&draft_file).ok();
        println!("Output updated with improved implementation.");
    } else if evaluation_response == "First result is better." {
        let now = FileTime::now();
        set_file_mtime(output_file, now).unwrap_or_else(|e| {
            eprintln!("Error updating mtime for {}: {}", output_file, e);
            exit(1);
        });
        
        fs::rename(&draft_file, &reject_file).ok();
        
        let diff_output = Command::new("diff")
            .arg("-c")
            .arg(output_file)
            .arg(&reject_file)
            .output();
            
        if let Ok(output) = diff_output {
            println!("Diff between original and rejected implementation:");
            io::stdout().write_all(&output.stdout).ok();
        }
        
        println!("Original implementation kept (deemed better).");
    } else {
        fs::rename(&draft_file, &reject_file).ok();
        eprintln!("Error: Unexpected evaluation response: '{}'", evaluation_response);
        exit(1);
    }
}

fn call_claude(api_key: &str, prompt: &str, max_tokens: u32, temperature: f64) -> String {
    let client = reqwest::blocking::Client::new();
    let payload = json!({
        "model": "claude-sonnet-4-20250514",
        "max_tokens": max_tokens,
        "temperature": temperature,
        "messages": [
            {
                "role": "user",
                "content": prompt
            }
        ]
    });

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("Content-Type", "application/json")
        .header("x-api-version", "2023-06-01")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&payload)
        .send()
        .unwrap_or_else(|e| {
            eprintln!("Error making API request: {}", e);
            exit(1);
        });

    if !response.status().is_success() {
        eprintln!("API request failed with status: {}", response.status());
        eprintln!("Response: {}", response.text().unwrap_or_default());
        exit(1);
    }

    let response_json: Value = response.json().unwrap_or_else(|e| {
        eprintln!("Error parsing JSON response: {}", e);
        exit(1);
    });

    response_json["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| {
            eprintln!("Error extracting text from response");
            exit(1);
        })
        .to_string()
}