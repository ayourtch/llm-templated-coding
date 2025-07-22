use std::env;
use std::fs;
use std::path::Path;
use std::process;
use serde_json::json;
use reqwest;
use tokio;
use filetime::{FileTime, set_file_mtime};

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() != 3 {
        eprintln!("Usage: {} <input_file> <output_file>", args[0]);
        process::exit(1);
    }
    
    let input_file = &args[1];
    let output_file = &args[2];
    let draft_file = format!("{}.draft", output_file);
    
    let api_key = match env::var("GROQ_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Error: GROQ_API_KEY environment variable not set");
            process::exit(1);
        }
    };
    
    let input_content = match fs::read_to_string(input_file) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading input file {}: {}", input_file, e);
            process::exit(1);
        }
    };
    
    let output_exists = Path::new(output_file).exists();
    let output_content = if output_exists {
        match fs::read_to_string(output_file) {
            Ok(content) => content,
            Err(_) => String::new(),
        }
    } else {
        String::new()
    };
    
    let prompt = if !output_exists || output_content.trim().is_empty() {
        format!("Please produce single output result, which would match the description below as well as you can:\n\n{}", input_content)
    } else {
        format!("Please verify that the description below (enclosed into <result-description></result-description>) matches the specimen (enclosed into <result-specimen></result-specimen>) as much as possible. If it does - then simply output the content of the result-specimen verbatim. If you find that there are imperfections in how result-specimen fulfills its purpose described in result-description, then improve it and output the full result, with your improvements. Do not delimit the result with anything, output it verbatim.\n\n<result-description>\n{}\n</result-description>\n\n<result-specimen>\n{}\n</result-specimen>", input_content, output_content)
    };
    
    println!("Sending request to Groq API...");
    
    let client = reqwest::Client::new();
    let response = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&json!({
            "model": "moonshotai/kimi-k2-instruct",
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.7,
            "max_tokens": 4096
        }))
        .send()
        .await;
    
    let response = match response {
        Ok(resp) => resp,
        Err(e) => {
            eprintln!("Error sending request to Groq API: {}", e);
            process::exit(1);
        }
    };
    
    let response_text = match response.text().await {
        Ok(text) => text,
        Err(e) => {
            eprintln!("Error reading response: {}", e);
            process::exit(1);
        }
    };
    
    let response_json: serde_json::Value = match serde_json::from_str(&response_text) {
        Ok(json) => json,
        Err(e) => {
            eprintln!("Error parsing JSON response: {}", e);
            eprintln!("Response was: {}", response_text);
            process::exit(1);
        }
    };
    
    let llm_response = response_json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();
    
    if let Err(e) = fs::write(&draft_file, &llm_response) {
        eprintln!("Error writing draft file: {}", e);
        process::exit(1);
    }
    
    println!("Draft saved to {}", draft_file);
    
    if !output_exists || output_content.trim().is_empty() {
        if let Err(e) = fs::write(output_file, &llm_response) {
            eprintln!("Error writing output file: {}", e);
            process::exit(1);
        }
        println!("Output written to {}", output_file);
        return;
    }
    
    println!("Evaluating results...");
    
    let evaluation_prompt = format!(
        "Please CAREFULLY evaluate the below description (enclosed into <result-description></result-description>), and two outputs corresponding to this description, first one enclosed into \"<first-result></first-result>\" and the second enclosed into \"<second-result></second-result>\", and evaluate which of the two is more precise and correct in implementing the description. Then, if the first result is better, output the phrase 'First result is better.', if the second result is better, output the phrase 'The second implementation is better.'. Output only one of the two phrases, and nothing else\n\n<result-description>\n{}\n</result-description>\n\n<first-result>\n{}\n</first-result>\n\n<second-result>\n{}\n</second-result>",
        input_content, output_content, llm_response
    );
    
    let eval_response = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&json!({
            "model": "moonshotai/kimi-k2-instruct",
            "messages": [
                {
                    "role": "user",
                    "content": evaluation_prompt
                }
            ],
            "temperature": 0.1,
            "max_tokens": 100
        }))
        .send()
        .await;
    
    let eval_response = match eval_response {
        Ok(resp) => resp,
        Err(e) => {
            eprintln!("Error sending evaluation request: {}", e);
            let rej_file = format!("{}.rej", output_file);
            if let Err(rename_err) = fs::rename(&draft_file, &rej_file) {
                eprintln!("Error renaming draft to rej: {}", rename_err);
            }
            process::exit(1);
        }
    };
    
    let eval_response_text = match eval_response.text().await {
        Ok(text) => text,
        Err(e) => {
            eprintln!("Error reading evaluation response: {}", e);
            let rej_file = format!("{}.rej", output_file);
            if let Err(rename_err) = fs::rename(&draft_file, &rej_file) {
                eprintln!("Error renaming draft to rej: {}", rename_err);
            }
            process::exit(1);
        }
    };
    
    let eval_json: serde_json::Value = match serde_json::from_str(&eval_response_text) {
        Ok(json) => json,
        Err(e) => {
            eprintln!("Error parsing evaluation JSON: {}", e);
            let rej_file = format!("{}.rej", output_file);
            if let Err(rename_err) = fs::rename(&draft_file, &rej_file) {
                eprintln!("Error renaming draft to rej: {}", rename_err);
            }
            process::exit(1);
        }
    };
    
    let evaluation = eval_json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim();
    
    println!("Evaluation result: {}", evaluation);
    
    match evaluation {
        "The second implementation is better." => {
            if let Err(e) = fs::write(output_file, &llm_response) {
                eprintln!("Error writing output file: {}", e);
                let rej_file = format!("{}.rej", output_file);
                if let Err(rename_err) = fs::rename(&draft_file, &rej_file) {
                    eprintln!("Error renaming draft to rej: {}", rename_err);
                }
                process::exit(1);
            }
            if let Err(e) = fs::remove_file(&draft_file) {
                eprintln!("Warning: Could not remove draft file: {}", e);
            }
            println!("Output updated with improved version");
        },
        "First result is better." => {
            let now = FileTime::now();
            if let Err(e) = set_file_mtime(output_file, now) {
                eprintln!("Error updating file mtime: {}", e);
            }
            let rej_file = format!("{}.rej", output_file);
            if let Err(e) = fs::rename(&draft_file, &rej_file) {
                eprintln!("Error renaming draft to rej: {}", e);
            }
            println!("Original version is better, file timestamp updated");
        },
        _ => {
            eprintln!("Unexpected evaluation response: {}", evaluation);
            let rej_file = format!("{}.rej", output_file);
            if let Err(e) = fs::rename(&draft_file, &rej_file) {
                eprintln!("Error renaming draft to rej: {}", e);
            }
            process::exit(1);
        }
    }
}