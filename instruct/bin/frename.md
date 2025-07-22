Please write a Rust program that takes three mandatory arguments: directory, regex to match, and a replacement string. Replacement string should allow $1, $2 from the match groups in regex.

The program should recursively look at all the files in the directory, and rename those that match a regex into the name specified in the replacement string.

There should be the following safeguards in place:
- no actual renames should take place (just the possible action should be printed) unless the user specifies "--do-rename" flag)
- the replacement string *must* resolve to different values for different files.

The target for the rename should be the same directory, i.e. regex "(.*?)\.txt" and replacement string "$1.bak" should recursively rename all .txt files into .bak version.

Also, a simpler format of regex being ".extension" and replacement being ".new_extension" should just do similar as the above - change the trailing extension of the files.

Do not stop the output until you output the whole program. The code MUST compile from the first shot.

For the implementation: Path does not have push() and pop() methods.

Do not use any markdown separators please.

Provides clear error messages and status updates.

Only updates the file when the new implementation is deemed better.

