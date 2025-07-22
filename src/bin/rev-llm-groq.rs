use std::{
    env,
    fs::{self, File},
    io::{self, Read, Write},
    path::Path,
    time::SystemTime,
};

use serde_json::json;

const GROQ_API_URL: &str = "https://api.groq.com/openai/v1/chat/completions";
const MODEL: &str = "moonshotai/kimi-k2-instruct";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <input_file> <output_file>", args[0]);
        std::process::exit(1);
    }

    let input_file = &args[1];
    let output_file = &args[2];

    let api_key = env::var("GROQ_API_KEY")
        .expect("Environment variable GROQ_API_KEY must be set");

    let specimen = fs::read_to_string(input_file)
        .unwrap_or_else(|_| panic!("Failed to read input file: {}", input_file));

    let output_path = Path::new(output_file);
    let draft_path = format!("{}.draft", output_file);
    let rej_path = format!("{}.rej", output_file);

    let (prompt, use_verification) = if !output_path.exists() || output_path.metadata()?.len() == 0 {
        (
            format!(
                "Please produce a detailed specification which will allow to recreate the implementation below from first principles:\n{}",
                specimen
            ),
            false,
        )
    } else {
        let specification = fs::read_to_string(output_file)?;
        (
            format!(
                "Please verify that the implementation below (enclosed into <result-specimen></result-specimen>) is accurately described by the specification (enclosed into <result-specification></result-specification>) as much as possible. If it does - then simply output the content of the result-specification verbatim. If you find that there are imperfections in how result-specification describes the specimen, then incrementally improve it and output the full result, with your improvements. Do not delimit the result with anything, output it verbatim.\n\n<result-specimen>\n{}\n</result-specimen>\n\n<result-specification>\n{}\n</result-specification>",
                specimen,
                specification
            ),
            true,
        )
    };

    let client = reqwest::Client::new();
    let response = client
        .post(GROQ_API_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&json!({
            "model": MODEL,
            "messages": [{"role": "user", "content": prompt}],
            "temperature": 0.7,
            "max_tokens": 4000
        }))
        .send()
        .await?;

    if !response.status().is_success() {
        eprintln!("API request failed: {}", response.status());
        std::process::exit(1);
    }

    let response_json: serde_json::Value = response.json().await?;
    let first_response = response_json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("Invalid response format")?;

    if !use_verification {
        fs::write(&draft_path, first_response)?;
        fs::rename(&draft_path, output_file)?;
        println!("Created new specification at {}", output_file);
        return Ok(());
    }

    fs::write(&draft_path, first_response)?;

    let eval_prompt = format!(
        "Please CAREFULLY evaluate the below specimen (enclosed into <result-specimen></result-specimen>), and two outputs corresponding to this description, first one enclosed into \"<first-specification></first-specification>\" and the second enclosed into \"<second-specification></second-specification>\", and evaluate which of the two is more precise and correct in describing the specimen. Then, if the first result is better, output the phrase 'First specification is better.', if the second description is better, output the phrase 'The second spec is better.'. Output only one of the two phrases, and nothing else\n\n<result-specimen>\n{}\n</result-specimen>\n\n<first-specification>\n{}\n</first-specification>\n\n<second-specification>\n{}\n</second-specification>",
        specimen,
        fs::read_to_string(output_file)?,
        first_response
    );

    let eval_response = client
        .post(GROQ_API_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&json!({
            "model": MODEL,
            "messages": [{"role": "user", "content": eval_prompt}],
            "temperature": 0.1,
            "max_tokens": 100
        }))
        .send()
        .await?;

    if !eval_response.status().is_success() {
        eprintln!("Evaluation API request failed: {}", eval_response.status());
        std::process::exit(1);
    }

    let eval_response_json: serde_json::Value = eval_response.json().await?;
    let eval_result = eval_response_json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("Invalid evaluation response format")?
        .trim();

    match eval_result {
        "The second spec is better." => {
            fs::rename(&draft_path, output_file)?;
            println!("Updated specification with improved version");
        }
        "First specification is better." => {
            fs::rename(&draft_path, &rej_path)?;
            filetime::set_file_mtime(output_file, filetime::FileTime::from_system_time(SystemTime::now()))?;
            println!("Kept original specification, moved draft to {}", rej_path);
        }
        _ => {
            eprintln!("Unexpected evaluation response: {}", eval_result);
            std::process::exit(1);
        }
    }

    Ok(())
}

