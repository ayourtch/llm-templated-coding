# Default binary
DEFAULT_BINARY = llm-groq

# Automatically discover all .md files in instruct/ and convert to corresponding .rs files in src/bin/
INSTRUCT_FILES := $(shell find instruct/ -name "*.md")
TARGETS := $(patsubst instruct/%.md,src/%.rs,$(INSTRUCT_FILES))

# Default target - build all discovered targets
all: $(TARGETS)

# Override binary for specific files (only specify if different from default)
src/bin/llm-groq.rs: BINARY = llm-claude
src/bin/llm-ollama-qwen.rs: BINARY = llm-claude

# Generic pattern rule: any .md in instruct/ creates corresponding .rs in src/bin/
src/%.rs: instruct/%.md
	cargo run --bin $(or $(BINARY),$(DEFAULT_BINARY)) -- instruct/$*.md src/$*.rs

.PHONY: all
