# Default binary
DEFAULT_BINARY = llm-groq

# Automatically discover all .txt files in instruct/ and convert to corresponding .rs files in src/bin/
INSTRUCT_FILES := $(shell find instruct/ -name "*.txt")
TARGETS := $(patsubst instruct/%.txt,src/%.rs,$(INSTRUCT_FILES))

# Default target - build all discovered targets
all: $(TARGETS)

# Override binary for specific files (only specify if different from default)
src/bin/llm-groq.rs: BINARY = llm-claude
src/bin/llm-ollama-qwen.rs: BINARY = llm-claude

# Generic pattern rule: any .txt in instruct/ creates corresponding .rs in src/bin/
src/%.rs: instruct/%.txt
	cargo run --bin $(or $(BINARY),$(DEFAULT_BINARY)) -- instruct/$*.txt src/$*.rs

.PHONY: all
