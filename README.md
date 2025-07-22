# llm-templated-coding PoC

After experimenting with vibe-coding for a bit, I realize it is a wrong approach (at least for me).

LLMs are like a vehicle with lane assist: you can use it almost autonomously for good straight stretches of the way,
but attempting to get from point A to B fully on autopilot will result in tears. Also, it is hard to control the context
that goes in.

The simple web-chat based workflow is somewhat better, but it is very hard to keep the prompts + their results in sync,
and iterate. Last but not least, it is really hard to switch the models to compare the results.

# Enter "LLM-templated coding"

This repository explores a new approach, whereby for a project source that lives in src/, there is a mirrored hierarchy in instruct/,
and whenever one changes a given .md file inside instruct and types "make", the llm edit command gets executed.

The LLM edit command accepts two arguments: .md file with instructions and a .rs file name that can contain existing data.

If the file does not exist, then the edit command will request the LLM to generate the .rs file based on the instructions in .md file,
this bit is not much new.

However, if the file exists, the llm file will perform two requests:

1) supply the template, the current .rs data, and the cargo check errors related to that .rs file to the LLM, and request to generate
a version better fitting the spec, or output the supplied one verbatim if it "thinks" it can not be improved.

2) upon retrieving that new version, it will generate cargo check on that one as well, and then submit a second request to the LLM,
with both sources and both sets of errors, and request the LLM to perform the selection of which one is better.

if the LLM selects the first output, then basically nothing happens. If the LLM selects the second output, then the file gets the new "better" content.

Note, that due to the way Makefile is setup, the usage of LLM is entirely optional - some files can and should be "manual", as simply writing it down in plain language would be a massive waste of space.

# What does it give us ? 

- clear understanding which files were LLM-assisted and which weren't (assuming you are disciplined about using the instruct/ directory rather than just copypasting.

- entanglement between the prompts and their results (or derivatives thereof) - as soon as LLM is reasonably "happy" that they correspond to prompts.
  The scheme allows one to perform bidirectional edits: both edit a spec, and edit the .rs file, as soon as the LLM does not
  dislike the edits too much - and it is trivial to correlate the prompt, and the output related to that prompt. In fact, the error checking function for groq
  has been "surgically supplied" by Claude.

- total control over the context, depending on the exact file. The "requirements" can in theory include also the source files, though I have not experimented with that yet.

- some amount of portability across LLMs: the interface is very high level, so one can easily swap out groq for claude, globally or per-file. This allows to have
  "tricky" file managed by the "smarter" models, and the simple ones be dealt with by a cheaper and smaller model. 

- one can also use the same technique to perform the reverse fixup as well: given the implementation and the spec, improve the spec.
  So far I have not seen huge success with it, but it might be a topic for future experimentation.

- I use this technique with Rust here, because I strongly believe it is one of the more LLM-friendly languages - strict typing and great compiler messages.
  But there is nothing that prevents you from using it with C, etc..

# Fun stats for nerds

Now, another curiosity - how verbose is the natural language ?

About 50% less verbose, it appears. Note, that of course all of the prompts are very under-specified, so the real "state" is also the whatever model is being used.

```bash
% ./target/debug/wcr instruct src
Pair: instruct/bin/wcr.md .md -> src/bin/wcr.rs .rs
  md:  bytes=1503, lines=22
  rs:  bytes=6738, lines=212
Pair: instruct/bin/rev-llm-groq.md .md -> src/bin/rev-llm-groq.rs .rs
  md:  bytes=3576, lines=42
  rs:  bytes=5500, lines=134
Pair: instruct/bin/llm-gemini.md .md -> src/bin/llm-gemini.rs .rs
  md:  bytes=1366, lines=17
  rs:  bytes=4493, lines=132
Pair: instruct/bin/llm-groq-2.md .md -> src/bin/llm-groq-2.rs .rs
  md:  bytes=4620, lines=56
  rs:  bytes=7933, lines=167
Pair: instruct/bin/llm-groq-3.md .md -> src/bin/llm-groq-3.rs .rs
  md:  bytes=4810, lines=59
  rs:  bytes=7786, lines=167
Pair: instruct/bin/llm-claude.md .md -> src/bin/llm-claude.rs .rs
  md:  bytes=3633, lines=40
  rs:  bytes=9560, lines=230
Pair: instruct/bin/frename.md .md -> src/bin/frename.rs .rs
  md:  bytes=1295, lines=22
  rs:  bytes=3190, lines=115
Pair: instruct/bin/llm-claude-2.md .md -> src/bin/llm-claude-2.rs .rs
  md:  bytes=3633, lines=40
  rs:  bytes=6578, lines=154
Pair: instruct/bin/lib/groq.md .md -> src/bin/lib/groq.rs .rs
  md:  bytes=837, lines=27
  rs:  bytes=2082, lines=82
Pair: instruct/bin/lib/ollama.md .md -> src/bin/lib/ollama.rs .rs
  md:  bytes=307, lines=6
  rs:  bytes=1552, lines=66
Pair: instruct/bin/lib/preprocess.md .md -> src/bin/lib/preprocess.rs .rs
  md:  bytes=1491, lines=41
  rs:  bytes=6381, lines=200
Pair: instruct/bin/llm-groq.md .md -> src/bin/llm-groq.rs .rs
  md:  bytes=3432, lines=37
  rs:  bytes=8050, lines=199
Pair: instruct/bin/next-llm.md .md -> src/bin/next-llm.rs .rs
  md:  bytes=805, lines=16
  rs:  bytes=3527, lines=113
Pair: instruct/bin/llm-groq-4.md .md -> src/bin/llm-groq-4.rs .rs
  md:  bytes=5743, lines=66
  rs:  bytes=10410, lines=229
Pair: instruct/bin/llm-groq-5.md .md -> src/bin/llm-groq-5.rs .rs
  md:  bytes=5627, lines=66
  rs:  bytes=10932, lines=236
Pair: instruct/bin/llm-ollama-qwen.md .md -> src/bin/llm-ollama-qwen.rs .rs
  md:  bytes=3467, lines=38
  rs:  bytes=4596, lines=103
=== Summary ===
Total .md files: bytes=46145, lines=595
Total matching .rs files: bytes=99308, lines=2539
Unmatched .rs files: 2 files, bytes=195, lines=12

=== Unmatched .rs files ===
src/bin/check.rs: bytes=144, lines=8
src/bin/lib/mod.rs: bytes=51, lines=4
```


