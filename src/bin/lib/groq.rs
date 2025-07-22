use std::env;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

pub struct Groq {
    client: Client,
    api_key: String,
}

impl Groq {
    pub fn new() -> Self {
        let api_key = env::var("GROQ_API_KEY").expect("GROQ_API_KEY must be set");
        Groq {
            client: Client::new(),
            api_key,
        }
    }

    pub fn evaluate(&self, prompt: &str) -> String {
        #[derive(Serialize)]
        struct RequestBody<'a> {
            model: &'a str,
            messages: Vec<Message<'a>>,
            max_tokens: u32,
        }

        #[derive(Serialize)]
        struct Message<'a> {
            role: &'a str,
            content: &'a str,
        }

        #[derive(Deserialize)]
        struct Response {
            choices: Vec<Choice>,
        }

        #[derive(Deserialize)]
        struct Choice {
            message: MessageContent,
        }

        #[derive(Deserialize)]
        struct MessageContent {
            content: String,
        }

        let body = RequestBody {
            model: "moonshotai/kimi-k2-instruct",
            messages: vec![Message {
                role: "user",
                content: prompt,
            }],
            max_tokens: 16384,
        };

        let response = self
            .client
            .post("https://api.groq.com/openai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .expect("Failed to send request");

        let text = response.text().expect("Failed to get response text");
        match serde_json::from_str::<Response>(&text) {
            Ok(parsed) => {
                parsed
                    .choices
                    .into_iter()
                    .next()
                    .expect("No choices returned")
                    .message
                    .content
            }
            Err(_) => {
                eprintln!("{}", text);
                panic!("Failed to parse JSON response");
            }
        }
    }
}