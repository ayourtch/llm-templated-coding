all: src/bin/llm-groq.rs src/bin/llm-groq-2.rs

src/bin/llm-groq.rs: instruct/llm-groq.txt
	cargo run --bin llm-groq -- instruct/llm-groq.txt src/bin/llm-groq.rs

src/bin/llm-groq-2.rs: instruct/llm-groq-2.txt
	cargo run --bin llm-claude -- instruct/llm-groq-2.txt src/bin/llm-groq-2.rs

.PHONY: all
