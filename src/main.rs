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
    
    if args.len() != 2 {
        eprintln!("Usage: {} <file_path>", args[0]);
        process::exit(1);
    }
    
    let file_path = &args[1];
    let api_key = env::var("GEMINI_API_KEY")
        .expect("GEMINI_API_KEY environment variable must be set");
    
    // Read the input file
    let file_content = fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read file {}: {}", file_path, e))?;
    
    // Prepare the API request
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash-exp:generateContent?key={}",
        api_key
    );
    
    let payload = json!({
        "contents": [{
            "parts": [{
                "text": file_content
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
    
    // Extract the generated text
    let generated_text = response_json
        ["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .ok_or("Failed to extract generated text from response")?;
    
    // Output to stdout
    println!("{}", generated_text);
    
    // Save to file
    let pid = process::id();
    let output_file = format!("/tmp/gemini-{}.txt", pid);
    
    fs::write(&output_file, generated_text)
        .map_err(|e| format!("Failed to write to {}: {}", output_file, e))?;
    
    eprintln!("Output saved to: {}", output_file);
    
    Ok(())
}

// Cargo.toml dependencies needed:
/*
[dependencies]
reqwest = { version = "0.11", features = ["json"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
*/
