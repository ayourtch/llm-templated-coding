use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::Path,
    process::{Command, Stdio},
};
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <input_description_file> <output_file>", args[0]);
        std::process::exit(1);
    }

    let input_desc_file = &args[1];
    let output_file = &args[2];
    let draft_file = format!("{}.draft", output_file);
    let reject_file = format!("{}.rej", output_file);

    let input_desc = fs::read_to_string(input_desc_file)?;
    let output_exists = Path::new(output_file).exists();
    let output_content = if output_exists {
        fs::read_to_string(output_file).unwrap_or_default()
    } else {
        String::new()
    };

    let prompt = if !output_exists || output_content.is_empty() {
        format!(
            "Please produce single output result, which would match the description below as well as you can:\n{}",
            input_desc
        )
    } else {
        format!(
            "Please verify that the description below (enclosed into <result-description></result-description>) matches the specimen (enclosed into <result-specimen></result-specimen>) as much as possible. If it does - then simply output the content of the result-specimen verbatim. If you find that there are imperfections in how result-specimen fulfills its purpose described in result-description, then improve it and output the full result, with your improvements, BUT without any side comments/observations. Do not delimit the result with anything, output it verbatim.\n<result-description>\n{}\n</result-description>\n<result-specimen>\n{}\n</result-specimen>",
            input_desc, output_content
        )
    };

    let response = call_ollama(&prompt, 0.7, 4000)?;
    fs::write(&draft_file, &response)?;

    let eval_prompt = format!(
        "Please CAREFULLY evaluate the below description (enclosed into <result-description></result-description>), and two outputs corresponding to this description, first one enclosed into \"<first-result></first-result>\" and the second enclosed into \"<second-result></second-result>\", and evaluate which of the two is more precise and correct in implementing the description. Then, if the first result is better, output the phrase 'First result is better.', if the second result is better, output the phrase 'The second implementation is better.'. Output only one of the two phrases, and NOTHING else - not your thoughts, not analysis.\n<result-description>\n{}\n</result-description>\n<first-result>\n{}\n</first-result>\n<second-result>\n{}\n</second-result>",
        input_desc, output_content, response
    );

    let eval_response = call_ollama(&eval_prompt, 0.1, 100)?;
    let eval_response = eval_response.trim();

    match eval_response {
        "The second implementation is better." => {
            fs::write(output_file, &response)?;
            println!("Updated {} with improved implementation", output_file);
        }
        "First result is better." => {
            filetime::set_file_mtime(output_file, filetime::FileTime::now())?;
            if Path::new(&draft_file).exists() {
                fs::rename(&draft_file, &reject_file)?;
                let output = Command::new("diff")
                    .args(&["-c", output_file, &reject_file])
                    .stdout(Stdio::piped())
                    .output()?;
                println!("Rejected changes. Diff:\n{}", String::from_utf8_lossy(&output.stdout));
            }
        }
        _ => {
            eprintln!("Unexpected evaluation response: {}", eval_response);
            std::process::exit(1);
        }
    }

    Ok(())
}

fn call_ollama(prompt: &str, temperature: f64, max_tokens: u32) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .post("http://localhost:11434/api/generate")
        .json(&json!({
            "model": "qwen2.5-coder:14b",
            "prompt": prompt,
            "stream": false,
            "options": {
                "temperature": temperature,
                "num_predict": max_tokens
            }
        }))
        .send()?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()).into());
    }

    let response_json: serde_json::Value = response.json()?;
    Ok(response_json["response"]
        .as_str()
        .unwrap_or("")
        .to_string())
}

