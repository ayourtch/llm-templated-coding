use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::SystemTime;

#[derive(Serialize)]
struct GroqRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct GroqResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <input_file> <output_file>", args[0]);
        std::process::exit(1);
    }

    let input_file = &args[1];
    let output_file = &args[2];
    let draft_file = format!("{}.draft", output_file);
    let reject_file = format!("{}.rej", output_file);

    let api_key = env::var("GROQ_API_KEY").expect("GROQ_API_KEY environment variable not set");

    println!("Reading input file: {}", input_file);
    let input_content = fs::read_to_string(input_file)?;

    let output_exists = Path::new(output_file).exists();
    let output_content = if output_exists {
        fs::read_to_string(output_file).unwrap_or_default()
    } else {
        String::new()
    };

    let prompt = if output_content.is_empty() {
        format!("Please produce single output result, which would match the description below as well as you can:\n\n{}", input_content)
    } else {
        format!("Please verify that the description below (enclosed into <result-description></result-description>) matches the specimen (enclosed into <result-specimen></result-specimen>) as much as possible. If it does - then simply output the content of the result-specimen verbatim. If you find that there are imperfections in how result-specimen fulfills its purpose described in result-description, then improve it and output the full result, with your improvements. Do not delimit the result with anything, output it verbatim.\n\n<result-description>\n{}\n</result-description>\n\n<result-specimen>\n{}\n</result-specimen>", input_content, output_content)
    };

    println!("Calling Groq API for initial generation...");
    let client = Client::new();
    let request = GroqRequest {
        model: "moonshotai/kimi-k2-instruct".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: prompt,
        }],
        temperature: 0.7,
        max_tokens: 4096,
    };

    let response = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()?;

    if !response.status().is_success() {
        eprintln!("API request failed: {}", response.status());
        eprintln!("Response: {}", response.text()?);
        std::process::exit(1);
    }

    let groq_response: GroqResponse = response.json()?;
    let llm_output = &groq_response.choices[0].message.content;

    println!("Writing draft file: {}", draft_file);
    fs::write(&draft_file, llm_output)?;

    if output_content.is_empty() {
        println!("No existing output file, writing new content to: {}", output_file);
        fs::write(output_file, llm_output)?;
        println!("Successfully created output file");
        return Ok(());
    }

    println!("Evaluating which implementation is better...");
    let evaluation_prompt = format!(
        "Please CAREFULLY evaluate the below description (enclosed into <result-description></result-description>), and two outputs corresponding to this description, first one enclosed into \"<first-result></first-result>\" and the second enclosed into \"<second-result></second-result>\", and evaluate which of the two is more precise and correct in implementing the description. Then, if the first result is better, output the phrase 'First result is better.', if the second result is better, output the phrase 'The second implementation is better.'. Output only one of the two phrases, and nothing else\n\n<result-description>\n{}\n</result-description>\n\n<first-result>\n{}\n</first-result>\n\n<second-result>\n{}\n</second-result>",
        input_content, output_content, llm_output
    );

    let eval_request = GroqRequest {
        model: "moonshotai/kimi-k2-instruct".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: evaluation_prompt,
        }],
        temperature: 0.1,
        max_tokens: 100,
    };

    let eval_response = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&eval_request)
        .send()?;

    if !eval_response.status().is_success() {
        eprintln!("Evaluation API request failed: {}", eval_response.status());
        eprintln!("Response: {}", eval_response.text()?);
        std::process::exit(1);
    }

    let eval_groq_response: GroqResponse = eval_response.json()?;
    let evaluation = eval_groq_response.choices[0].message.content.trim();

    println!("Evaluation result: {}", evaluation);

    match evaluation {
        "The second implementation is better." => {
            println!("Writing improved content to: {}", output_file);
            fs::write(output_file, llm_output)?;
            println!("Successfully updated output file with improved implementation");
        }
        "First result is better." => {
            println!("Existing implementation is better, updating modification time only");
            let file = fs::OpenOptions::new()
                .write(true)
                .open(output_file)?;
            file.set_modified(SystemTime::now())?;
            fs::rename(&draft_file, &reject_file)?;
            println!("Draft file renamed to: {}", reject_file);
        }
        _ => {
            eprintln!("Unexpected evaluation response: {}", evaluation);
            std::process::exit(1);
        }
    }

    Ok(())
}