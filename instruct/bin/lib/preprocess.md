# Target deliverable

A Rust library that implements preprocessing of text in order to perform includes.

# Library Interface

Code would expose a single function:

- fn preprocess(file_name: &str) -> String  

The work that this function does:

  Read the file with the given file_name, and look through its contents.
  If there is a string '{!filename!}' in the file, replace that text with the preprocessed contents of that filename path.
  If the last line of the included file does not end with a newline character, add it.
  File paths are relative to the current directory if they are not fully qualified.

There should be a maximum include depth of 32 - if it is reached, the file should not be processed, but rather a string "TOO MUCH NESTED INCLUDES" should be inserted in place of contents.

# Testing

- Please implement the 5-10 tests that verify the functioning of the library
- Use tempfile crate for creating temporary files for testing.

# Your implementation details

- Do not stop the output until you output the whole program. The code MUST compile from the first shot.
- Do not use any markdown separators please.
- If you want to include any other content, like suggestions on what to put in other files, include inside multiline comment and explain accordingly ("/* */")
- IMPORTANT: the result MUST compile!

