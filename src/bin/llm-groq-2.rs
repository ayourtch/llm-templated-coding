use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::SystemTime;
use serde_json::{json, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <input_file> <output_file>", args[0]);
        std::process::exit(1);
    }

    let input_file = &args[1];
    let output_file = &args[2];
    let draft_file = format!("{}.draft", output_file);
    let rejected_file = format!("{}.rej", output_file);

    let api_key = env::var("GROQ_API_KEY")
        .map_err(|_| "GROQ_API_KEY environment variable not set")?;

    let input_content = fs::read_to_string(input_file)
        .map_err(|e| format!("Failed to read input file '{}': {}", input_file, e))?;

    let output_exists = Path::new(output_file).exists();
    let output_content = if output_exists {
        fs::read_to_string(output_file).unwrap_or_default()
    } else {
        String::new()
    };

    let prompt = if !output_exists || output_content.trim().is_empty() {
        println!("Output file doesn't exist or is empty. Using initial prompt.");
        format!("Please produce single output result, which would match the description below as well as you can:\n\n{}", input_content)
    } else {
        println!("Output file exists with content. Using improvement prompt.");
        format!("Please verify that the description below (enclosed into <result-description></result-description>) matches the specimen (enclosed into <result-specimen></result-specimen>) as much as possible. If it does - then simply output the content of the result-specimen verbatim. If you find that there are imperfections in how result-specimen fulfills its purpose described in result-description, then improve it and output the full result, with your improvements. Do not delimit the result with anything, output it verbatim.\n\n<result-description>\n{}\n</result-description>\n\n<result-specimen>\n{}\n</result-specimen>", input_content, output_content)
    };

    println!("Sending request to Groq API...");
    let llm_response = call_groq_api(&api_key, &prompt, 1.0, 4096)?;
    
    println!("Saving response to draft file...");
    fs::write(&draft_file, &llm_response)
        .map_err(|e| format!("Failed to write draft file: {}", e))?;

    if !output_exists || output_content.trim().is_empty() {
        println!("Initial generation complete. Writing to output file.");
        fs::write(output_file, &llm_response)
            .map_err(|e| format!("Failed to write output file: {}", e))?;
        return Ok(());
    }

    println!("Evaluating which result is better...");
    let evaluation_prompt = format!(
        "Please CAREFULLY evaluate the below description (enclosed into <result-description></result-description>), and two outputs corresponding to this description, first one enclosed into \"<first-result></first-result>\" and the second enclosed into \"<second-result></second-result>\", and evaluate which of the two is more precise and correct in implementing the description. Then, if the first result is better, output the phrase 'First result is better.', if the second result is better, output the phrase 'The second implementation is better.'. Output only one of the two phrases, and nothing else\n\n<result-description>\n{}\n</result-description>\n\n<first-result>\n{}\n</first-result>\n\n<second-result>\n{}\n</second-result>",
        input_content, output_content, llm_response
    );

    let evaluation_response = call_groq_api(&api_key, &evaluation_prompt, 0.1, 100)?;
    let evaluation_response = evaluation_response.trim();

    println!("Evaluation response: '{}'", evaluation_response);

    match evaluation_response {
        "The second implementation is better." => {
            println!("New implementation is better. Updating output file.");
            fs::write(output_file, &llm_response)
                .map_err(|e| format!("Failed to write output file: {}", e))?;
            println!("Output file updated successfully.");
        }
        "First result is better." => {
            println!("Original implementation is better. Updating file modification time.");
            let file = fs::OpenOptions::new().write(true).open(output_file)
                .map_err(|e| format!("Failed to open output file for touch: {}", e))?;
            file.set_modified(SystemTime::now())
                .map_err(|e| format!("Failed to update file modification time: {}", e))?;
            
            println!("Renaming draft to rejected file.");
            fs::rename(&draft_file, &rejected_file)
                .map_err(|e| format!("Failed to rename draft to rejected: {}", e))?;
        }
        _ => {
            println!("Renaming draft to rejected file due to unexpected evaluation response.");
            fs::rename(&draft_file, &rejected_file)
                .map_err(|e| format!("Failed to rename draft to rejected: {}", e))?;
            return Err(format!("Unexpected evaluation response: '{}'", evaluation_response).into());
        }
    }

    Ok(())
}

fn call_groq_api(api_key: &str, prompt: &str, temperature: f64, max_tokens: u32) -> Result<String, Box<dyn std::error::Error>> {
    let client = std::process::Command::new("curl")
        .arg("-s")
        .arg("-X")
        .arg("POST")
        .arg("https://api.groq.com/openai/v1/chat/completions")
        .arg("-H")
        .arg("Content-Type: application/json")
        .arg("-H")
        .arg(format!("Authorization: Bearer {}", api_key))
        .arg("-d")
        .arg(json!({
            "model": "moonshotai/kimi-k2-instruct",
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": temperature,
            "max_tokens": max_tokens
        }).to_string())
        .output()
        .map_err(|e| format!("Failed to execute curl command: {}", e))?;

    if !client.status.success() {
        return Err(format!("API request failed with status: {}", client.status).into());
    }

    let response_text = String::from_utf8(client.stdout)
        .map_err(|e| format!("Failed to parse API response as UTF-8: {}", e))?;

    let response_json: Value = serde_json::from_str(&response_text)
        .map_err(|e| format!("Failed to parse API response as JSON: {}", e))?;

    let content = response_json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("Failed to extract content from API response")?;

    Ok(content.to_string())
}