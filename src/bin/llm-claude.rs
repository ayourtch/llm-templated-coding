use std::env;
use std::fs;
use std::io::{self, Write};
use std::process;
use std::time::SystemTime;
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
    let draft_file = format!("{}.draft", output_file);
    let api_key = env::var("ANTHROPIC_API_KEY")
        .expect("ANTHROPIC_API_KEY environment variable must be set");
    
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
            "Please verify that the description below (enclosed into <result-description></result-description>) matches the specimen (enclosed into <result-specimen></result-specimen>) as much as possible. If it does - then simply output the content of the result-specimen verbatim. If you find that there are imperfections in how result-specimen fulfills its purpose described in result-description, then improve it and output the full result, with your improvements, BUT without any side comments/observations. Do not delimit the result with anything, output it verbatim.\n\n<result-description>\n{}\n</result-description>\n\n<result-specimen>\n{}\n</result-specimen>",
            input_content,
            existing_output
        )
    } else {
        format!(
            "Please produce single output result, which would match the description below as well as you can:\n\n{}",
            input_content
        )
    };
    
    // Prepare the API request for Claude
    let url = "https://api.anthropic.com/v1/messages";
    
    let payload = json!({
        "model": "claude-sonnet-4-20250514",
        "max_tokens": 8192,
        "temperature": 0.7,
        "messages": [{
            "role": "user",
            "content": prompt
        }]
    });
    
    // Make the API call
    let client = reqwest::Client::new();
    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&payload)
        .send()
        .await?;
    
    if !response.status().is_success() {
        let error_text = response.text().await?;
        eprintln!("API request failed: {}", error_text);
        process::exit(1);
    }
    
    let response_json: Value = response.json().await?;
    
    // Extract the generated text from Claude's response format
    let content_array = response_json["content"].as_array()
        .ok_or("Failed to get content array from response")?;
    
    let mut generated_text = String::new();
    for content_block in content_array {
        if content_block["type"] == "text" {
            if let Some(text) = content_block["text"].as_str() {
                generated_text.push_str(text);
            }
        }
    }
    
    if generated_text.is_empty() {
        return Err("No text content found in response".into());
    }
    
    // Save the response to draft file
    fs::write(&draft_file, &generated_text)
        .map_err(|e| format!("Failed to write draft file {}: {}", draft_file, e))?;
    eprintln!("Draft saved to: {}", draft_file);
    
    // If output file exists and has content, we need to compare
    if output_exists_and_not_empty {
        let existing_output = fs::read_to_string(output_file)
            .map_err(|e| format!("Failed to read existing output file {}: {}", output_file, e))?;
        
        // Prepare evaluation prompt
        let evaluation_prompt = format!(
            "Please CAREFULLY evaluate the below description (enclosed into <result-description></result-description>), and two outputs corresponding to this description, first one enclosed into \"<first-result></first-result>\" and the second enclosed into \"<second-result></second-result>\", and evaluate which of the two is more precise and correct in implementing the description. Then, if the first result is better, output the phrase 'First result is better.', if the second result is better, output the phrase 'The second implementation is better.'. Output only one of the two phrases, and NOTHING else - not your thoughts, not analysis.\n\n<result-description>\n{}\n</result-description>\n\n<first-result>\n{}\n</first-result>\n\n<second-result>\n{}\n</second-result>",
            input_content,
            existing_output,
            generated_text
        );
        
        // Make evaluation API call
        let eval_payload = json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 100,
            "temperature": 0.1,
            "messages": [{
                "role": "user",
                "content": evaluation_prompt
            }]
        });
        
        let eval_response = client
            .post(url)
            .header("Content-Type", "application/json")
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&eval_payload)
            .send()
            .await?;
        
        if !eval_response.status().is_success() {
            let error_text = eval_response.text().await?;
            eprintln!("Evaluation API request failed: {}", error_text);
            process::exit(1);
        }
        
        let eval_response_json: Value = eval_response.json().await?;
        
        // Extract evaluation result from Claude's response format
        let eval_content_array = eval_response_json["content"].as_array()
            .ok_or("Failed to get evaluation content array from response")?;
        
        let mut evaluation_result = String::new();
        for content_block in eval_content_array {
            if content_block["type"] == "text" {
                if let Some(text) = content_block["text"].as_str() {
                    evaluation_result.push_str(text);
                }
            }
        }
        
        let evaluation_result = evaluation_result.trim();
        eprintln!("Evaluation result: {}", evaluation_result);
        
        match evaluation_result {
            "First result is better." => {
                eprintln!("Keeping existing output file unchanged, updating mtime.");
                // Update mtime on the output file
                filetime::set_file_mtime(output_file, filetime::FileTime::now())?;
                
                // Rename draft file to .rej since it wasn't accepted
                let reject_file = format!("{}.rej", output_file);
                fs::rename(&draft_file, &reject_file)
                    .map_err(|e| format!("Failed to rename draft file to reject: {}", e))?;
                eprintln!("Draft file renamed to: {}", reject_file);
                
                return Ok(());
            },
            "The second implementation is better." => {
                eprintln!("Updating output file with new content.");
                // Continue to write the new content below
            },
            _ => {
                eprintln!("Error: Unexpected evaluation result: '{}'", evaluation_result);
                eprintln!("Expected either 'First result is better.' or 'The second implementation is better.'");
                // Keep draft file for diagnostic purposes
                eprintln!("Draft file kept at: {} for diagnostic purposes", draft_file);
                process::exit(1);
            }
        }
    }
    
    // Write the result to the output file
    fs::write(output_file, generated_text)
        .map_err(|e| format!("Failed to write to {}: {}", output_file, e))?;
    
    eprintln!("Output written to: {}", output_file);
    
    // Remove draft file since its content was accepted
    fs::remove_file(&draft_file).ok();
    
    Ok(())
}

// Cargo.toml dependencies needed:
/*
[dependencies]
reqwest = { version = "0.11", features = ["json"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
filetime = "0.2"
*/
