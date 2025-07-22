use std::env;
use std::fs;
use std::process;
use reqwest;
use serde_json::{json, Value};
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Argument Parsing
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <input_file> <output_file>", args[0]);
        process::exit(1);
    }
    let input_file = &args[1];
    let output_file = &args[2];

    // 2. Get API Key from environment variable
    let api_key = match env::var("GEMINI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Error: GEMINI_API_KEY environment variable must be set.");
            process::exit(1);
        }
    };

    // 3. Read input file
    let input_content = fs::read_to_string(input_file)
        .map_err(|e| format!("Failed to read input file {}: {}", input_file, e))?;

    // 4. Determine prompt based on output file state
    let output_exists_and_not_empty = fs::metadata(output_file)
        .map(|metadata| metadata.len() > 0)
        .unwrap_or(false);

    let prompt = if output_exists_and_not_empty {
        let existing_output = fs::read_to_string(output_file)
            .map_err(|e| format!("Failed to read output file {}: {}", output_file, e))?;
        
        format!(
            "Please verify that the description below (enclosed into <result-description></result-description>) matches the specimen (enclosed into <result-specimen></result-specimen>) as much as possible. If it does - then simply output the content of the result-specimen verbatim. If you find that there are imperfections in how result-specimen fulfills its purpose described in result-description, then improve it and output the full result, with your improvements. Do not delimit the result with anything, output it verbatim.\n\n<result-description>\n{}\n</result-description>\n\n<result-specimen>\n{}\n</result-specimen>",
            input_content,
            existing_output
        )
    } else {
        format!(
            "Please produce single output result, which would match the description below as well as you can:\n\n{}",
            input_content
        )
    };

    // 5. Prepare and send API request
    let url = "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-pro:generateContent";
    
    let payload = json!({
        "contents": [{
            "parts": [{
                "text": prompt
            }]
        }],
        "generationConfig": {
            "temperature": 0.7,
            "topK": 40,
            "topP": 0.95,
            "maxOutputTokens": 8192
        }
    });

    let client = reqwest::Client::new();
    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .header("X-goog-api-key", &api_key)
        .json(&payload)
        .send()
        .await?;

    // 6. Handle API response
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await?;
        eprintln!("API request failed with status {}: {}", status, error_text);
        process::exit(1);
    }

    let response_json: Value = response.json().await?;
    
    // 7. Extract text from response robustly
    let parts_array = response_json
        .get("candidates")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("content"))
        .and_then(|c| c.get("parts"))
        .and_then(|p| p.as_array());

    let mut generated_text = String::new();
    if let Some(parts) = parts_array {
        for part in parts {
            if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                generated_text.push_str(text);
            }
        }
    }

    if generated_text.is_empty() {
        eprintln!("Error: No text content found in the API response.");
        eprintln!("Full response: {}", serde_json::to_string_pretty(&response_json)?);
        process::exit(1);
    }
    
    // 8. Write result to output file
    fs::write(output_file, &generated_text)
        .map_err(|e| format!("Failed to write to {}: {}", output_file, e))?;

    eprintln!("Successfully wrote response to {}", output_file);

    Ok(())
}

// To compile this code, create a Cargo.toml file with these dependencies:
/*
[package]
name = "gemini_caller"
version = "0.1.0"
edition = "2021"

[dependencies]
reqwest = { version = "0.12", features = ["json"] }
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }
*/