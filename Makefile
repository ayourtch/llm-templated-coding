all: src/bin/llm-groq.rs src/bin/llm-groq-2.rs src/bin/llm-claude.rs src/bin/llm-ollama-qwen.rs

src/bin/llm-groq.rs: instruct/llm-groq.txt
	cargo run --bin llm-groq -- instruct/llm-groq.txt src/bin/llm-groq.rs

src/bin/llm-ollama-qwen.rs: instruct/llm-ollama-qwen.txt
	cargo run --bin llm-groq -- instruct/llm-ollama-qwen.txt src/bin/llm-ollama-qwen.rs

src/bin/llm-groq-2.rs: instruct/llm-groq-2.txt
	cargo run --bin llm-claude -- instruct/llm-groq-2.txt src/bin/llm-groq-2.rs

src/bin/llm-claude.rs: instruct/llm-claude.txt
	cargo run --bin llm-groq -- instruct/llm-claude.txt src/bin/llm-claude.rs

.PHONY: all
