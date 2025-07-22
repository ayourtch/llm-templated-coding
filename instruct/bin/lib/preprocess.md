# Target deliverable

A Rust library that implements preprocessing of text in order to perform includes.

# Library Interface

Code would expose a single function:

- fn preprocess(file_name: &str) -> String  

The work that this function does:

  Read the file with the given file_name, and look through its contents.
  Find all occurrences of the pattern {!filename!} where filename is a path to another file.
  Replace each such occurrence with the preprocessed contents of that filename path.
  Relative paths are resolved relative to the directory containing the file being processed.
  After processing all includes, ensure the final result ends with a newline character by adding one if missing.
  If an include depth of 32 is reached, insertion of "TOO MUCH NESTED INCLUDES" should occur instead of processing the file further.

# Testing

- Please implement the 5-10 tests that verify the functioning of the library
- Use tempfile crate for creating temporary files for testing.

# Your implementation details

- Do not stop the output until you output the whole program. The code MUST compile from the first shot.
- Do not use any markdown separators please.
- If you want to include any other content, like suggestions on what to put in other files, include inside multiline comment and explain accordingly ("/* */")
- IMPORTANT: the result MUST compile!

/*
Cargo.toml:
[package]
name = "text_preprocessor"
version = "0.1.0"
edition = "2021"

[dependencies]
tempfile = "3.10"
*/
