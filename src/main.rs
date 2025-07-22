use std::env;
use std::fs;
use std::io::{self, Write};
use std::process;
use reqwest;
use serde_json::{json, Value};
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() != 3 {
        eprintln!("Usage: {} <input_file> <output_file>", args[0]);
        process::exit(1);
    }
    
    let input_file = &args[1];
    let output_file = &args[2];
    let api_key = env::var("GEMINI_API_KEY")
        .expect("GEMINI_API_KEY environment variable must be set");
    
    // Read the input file
    let input_content = fs::read_to_string(input_file)
        .map_err(|e| format!("Failed to read input file {}: {}", input_file, e))?;
    
    // Check if output file exists and is non-empty
    let output_exists_and_not_empty = fs::metadata(output_file)
        .map(|metadata| metadata.len() > 0)
        .unwrap_or(false);
    
    let prompt = if output_exists_and_not_empty {
        // Read existing output file content
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
    
    // Prepare the API request
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash-exp:generateContent?key={}",
        api_key
    );
    
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
    
    // Make the API call
    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await?;
    
    if !response.status().is_success() {
        let error_text = response.text().await?;
        eprintln!("API request failed: {}", error_text);
        process::exit(1);
    }
    
    let response_json: Value = response.json().await?;
    
    // Extract the generated text from all parts
    let candidate = &response_json["candidates"][0]["content"]["parts"];
    let parts_array = candidate.as_array()
        .ok_or("Failed to get parts array from response")?;
    
    let mut generated_text = String::new();
    for part in parts_array {
        if let Some(text) = part["text"].as_str() {
            generated_text.push_str(text);
        }
    }
    
    if generated_text.is_empty() {
        return Err("No text content found in response".into());
    }
    
    // Write the result to the output file
    fs::write(output_file, generated_text)
        .map_err(|e| format!("Failed to write to {}: {}", output_file, e))?;
    
    eprintln!("Output written to: {}", output_file);
    
    Ok(())
}

// Cargo.toml dependencies needed:
/*
[dependencies]
reqwest = { version = "0.11", features = ["json"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
*/
