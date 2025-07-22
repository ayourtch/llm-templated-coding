# Define the targets
TARGETS = src/bin/llm-groq.rs src/bin/llm-groq-2.rs src/bin/llm-claude.rs src/bin/llm-ollama-qwen.rs src/bin/llm-claude-2.rs

# Default target
all: $(TARGETS)

# Set the binary to use for each target
src/bin/llm-groq.rs: BINARY = llm-groq
src/bin/llm-ollama-qwen.rs: BINARY = llm-groq
src/bin/llm-claude.rs: BINARY = llm-groq
src/bin/llm-groq-2.rs: BINARY = llm-claude
src/bin/llm-claude-2.rs: BINARY = llm-claude

# Generic pattern rule
src/bin/%.rs: instruct/%.txt
	cargo run --bin $(BINARY) -- instruct/$*.txt src/bin/$*.rs

.PHONY: all
