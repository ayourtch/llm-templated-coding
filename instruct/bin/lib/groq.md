# Target deliverable

A Rust library that implements calling Groq model "moonshotai/kimi-k2-instruct" with auth token from an environment variable GROQ_API_KEY. 

# Library Interface

Code would define "struct Groq" with opaque contents.

It would define Impl LlmCall with the following methods:

- fn new()
  return the new instance of Groq API caller

- fn evaluate(prompt: &str) -> String  
  perform the evaluation of "prompt" within LLM and return the result.
  max-tokens should be set to 65536.

# Your implementation details

- Do not stop the output until you output the whole program. The code MUST compile from the first shot.
- Do not use any markdown separators please.

# Used libraries

- reqwest: for HTTP request handling

