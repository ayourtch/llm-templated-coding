use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::Path,
    process,
    time::SystemTime,
};

use reqwest::{blocking::Client, header};
use serde_json::json;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <input-file> <output-file>", args[0]);
        process::exit(1);
    }

    let input_path = &args[1];
    let output_path = &args[2];

    let api_key = env::var("GROQ_API_KEY")
        .expect("Environment variable GROQ_API_KEY must be set");

    let client = Client::new();
    let model = "moonshotai/kimi-k2-instruct";

    let mut input_file = File::open(input_path)?;
    let mut description = String::new();
    input_file.read_to_string(&mut description)?;

    let output_exists = Path::new(output_path).exists();
    let output_empty = if output_exists {
        fs::metadata(output_path)?.len() == 0
    } else {
        true
    };

    let prompt = if !output_exists || output_empty {
        format!(
            "Please produce single output result, which would match the description below as well as you can:\n\n{}",
            description
        )
    } else {
        let mut existing_content = String::new();
        File::open(output_path)?.read_to_string(&mut existing_content)?;

        format!(
            "Please verify that the description below (enclosed into <result-description></result-description>) matches the specimen (enclosed into <result-specimen></result-specimen>) as much as possible. If it does - then simply output the content of the result-specimen verbatim. If you find that there are imperfections in how result-specimen fulfills its purpose described in result-description, then improve it and output the full result, with your improvements. Do not delimit the result with anything, output it verbatim.\n\n<result-description>\n{}\n</result-description>\n\n<result-specimen>\n{}\n</result-specimen>",
            description.trim(),
            existing_content.trim()
        )
    };

    let draft_path = format!("{}.draft", output_path);
    let response = send_request(&client, &api_key, model, &prompt, 0.7, 2048)?;
    save_file(&draft_path, &response)?;

    let mut original_content = String::new();
    if output_exists && !output_empty {
        File::open(output_path)?.read_to_string(&mut original_content)?;
    }

    let eval_prompt = format!(
        "Please CAREFULLY evaluate the below description (enclosed into <result-description></result-description>), and two outputs corresponding to this description, first one enclosed into \"<first-result></first-result>\" and the second enclosed into \"<second-result></second-result>\", and evaluate which of the two is more precise and correct in implementing the description. Then, if the first result is better, output the phrase 'First result is better.', if the second result is better, output the phrase 'The second implementation is better.'. Output only one of the two phrases, and nothing else\n\n<result-description>\n{}\n</result-description>\n\n<first-result>\n{}\n</first-result>\n\n<second-result>\n{}\n</second-result>",
        description.trim(),
        original_content.trim(),
        response.trim()
    );

    let eval_response = send_request(&client, &api_key, model, &eval_prompt, 0.1, 100)?;
    let trimmed_response = eval_response.trim();

    match trimmed_response {
        "The second implementation is better." => {
            fs::rename(&draft_path, output_path)?;
            println!("Updated {}", output_path);
        }
        "First result is better." => {
            let _ = File::create(&draft_path)?.set_modified(SystemTime::now())?;
            fs::rename(&draft_path, format!("{}.rej", output_path))?;
            println!("Kept original content, renamed draft to .rej");
        }
        _ => {
            eprintln!("Unexpected evaluation response: {}", trimmed_response);
            process::exit(1);
        }
    }

    Ok(())
}

fn send_request(
    client: &Client,
    api_key: &str,
    model: &str,
    prompt: &str,
    temperature: f32,
    max_tokens: u32,
) -> Result<String, Box<dyn std::error::Error>> {
    let body = json!({
        "model": model,
        "messages": [
            {
                "role": "user",
                "content": prompt
            }
        ],
        "temperature": temperature,
        "max_tokens": max_tokens
    });

    let response = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header(header::AUTHORIZATION, format!("Bearer {}", api_key))
        .header(header::CONTENT_TYPE, "application/json")
        .json(&body)
        .send()?;

    if !response.status().is_success() {
        return Err(format!("API request failed: {}", response.status()).into());
    }

    let json: serde_json::Value = response.json()?;
    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("Invalid response format")?;

    Ok(content.to_string())
}

fn save_file(path: &str, content: &str) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}