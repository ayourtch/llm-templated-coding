all: src/bin/llm-groq.rs

src/bin/llm-groq.rs: instruct/llm-groq.txt
	cargo run --bin llm-groq -- instruct/llm-groq.txt src/bin/llm-groq.rs

.PHONY: all
