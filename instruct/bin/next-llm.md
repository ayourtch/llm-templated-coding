Please write a Rust program as below.

The program takes one argument, which is the string, say it is "foo".

It should look into both instruct/bin/ and src/bin/ subdirectories for the files named "llm-foo-<number>.md" and "llm-foo-<number>.rs" respectively and find the biggest number value.

Then it should store that value in current_num variable, and assign next_num value +1 that.

After that, it should copy the llm-foo-<current_num>.md into llm-foo-<next_num>.md, and llm-foo-<current_num>.rs into llm-foo-<next_num>.rs in their respective directories, and perform git commit -m "first commit for llm-foo-<next_num>" for both files.

Do not use any markdown separators please.

Provides clear error messages and status updates.

Only updates the file when the new implementation is deemed better.

