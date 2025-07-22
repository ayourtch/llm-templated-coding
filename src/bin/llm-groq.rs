use std::env;
use std::fs;
use std::io::{self, Write};
use std::process;
use std::time::SystemTime;
use reqwest;
use serde_json::{json, Value};
use tokio;
use filetime::FileTime;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() != 3 {
        eprintln!("Usage: {} <input_file> <output_file>", args[0]);
        process::exit(1);
    }
    
    let input_file = &args[1];
    let output_file = &args[2];
    let api_key = env::var("GROQ_API_KEY")
        .expect("GROQ_API_KEY environment variable must be set");
    
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
    
    // Prepare the API request for Groq
    let url = "https://api.groq.com/openai/v1/chat/completions";
    
    let payload = json!({
        "model": "moonshotai/kimi-k2-instruct",
        "messages": [{
            "role": "user",
            "content": prompt
        }],
        "temperature": 0.7,
        "max_tokens": 8192,
        "top_p": 0.95,
        "stream": false
    });
    
    // Make the API call
    let client = reqwest::Client::new();
    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&payload)
        .send()
        .await?;
    
    if !response.status().is_success() {
        let error_text = response.text().await?;
        eprintln!("API request failed: {}", error_text);
        process::exit(1);
    }
    
    let response_json: Value = response.json().await?;
    
    // Extract the generated text from OpenAI-compatible format
    let generated_text = response_json
        ["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("Failed to extract generated text from response")?;
    
    // Save draft file
    let draft_file = format!("{}.draft", output_file);
    fs::write(&draft_file, generated_text)
        .map_err(|e| format!("Failed to write draft file {}: {}", draft_file, e))?;
    
    // If output file exists and has content, we need to compare
    if output_exists_and_not_empty {
        let existing_output = fs::read_to_string(output_file)
            .map_err(|e| format!("Failed to read existing output file {}: {}", output_file, e))?;
        
        // Prepare evaluation prompt
        let evaluation_prompt = format!(
            "Please CAREFULLY evaluate the below description (enclosed into <result-description></result-description>), and two outputs corresponding to this description, first one enclosed into \"<first-result></first-result>\" and the second enclosed into \"<second-result></second-result>\", and evaluate which of the two is more precise and correct in implementing the description. Then, if the first result is better, output the phrase 'First result is better.', if the second result is better, output the phrase 'The second implementation is better.'. Output only one of the two phrases, and nothing else\n\n<result-description>\n{}\n</result-description>\n\n<first-result>\n{}\n</first-result>\n\n<second-result>\n{}\n</second-result>",
            input_content,
            existing_output,
            generated_text
        );
        
        // Make evaluation API call
        let eval_payload = json!({
            "model": "moonshotai/kimi-k2-instruct",
            "messages": [{
                "role": "user",
                "content": evaluation_prompt
            }],
            "temperature": 0.1,
            "max_tokens": 100,
            "top_p": 0.95,
            "stream": false
        });
        
        let eval_response = client
            .post(url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&eval_payload)
            .send()
            .await?;
        
        if !eval_response.status().is_success() {
            let error_text = eval_response.text().await?;
            eprintln!("Evaluation API request failed: {}", error_text);
            process::exit(1);
        }
        
        let eval_response_json: Value = eval_response.json().await?;
        let evaluation_result = eval_response_json
            ["choices"][0]["message"]["content"]
            .as_str()
            .ok_or("Failed to extract evaluation result from response")?
            .trim();
        
        eprintln!("Evaluation result: {}", evaluation_result);
        
        match evaluation_result {
            "First result is better." => {
                eprintln!("Keeping existing output file unchanged.");
                // Update mtime as requested
                let file_time = FileTime::from_system_time(SystemTime::now());
                filetime::set_file_mtime(output_file, file_time)?;
                return Ok(());
            },
            "The second implementation is better." => {
                eprintln!("Updating output file with new content.");
                // Continue to write the new content below
            },
            _ => {
                eprintln!("Error: Unexpected evaluation result: '{}'", evaluation_result);
                eprintln!("Expected either 'First result is better.' or 'The second implementation is better.'");
                process::exit(1);
            }
        }
    }
    
    // Write the result to the output file
    fs::write(output_file, generated_text)
        .map_err(|e| format!("Failed to write to {}: {}", output_file, e))?;
    
    // Clean up draft file if it exists
    let _ = fs::remove_file(draft_file);
    
    eprintln!("Output written to: {}", output_file);
    
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