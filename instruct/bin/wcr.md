Please write a Rust program that takes a mandatory argument: two directory names.

The first directory hierarchy contains .md files, which describe the matching .rs files in the second hierarchy: a file name is constructed by removing .md from the end and appending .rs.
Both the .md and .rs file hierarchies have identical "shape": if there is a file md_dir/subdir/subdir2/file.md, its .rs peer is at rs_dir/subdir/subdir2/file.rs, so take this into account
when building the .rs file path from .md file path: first replace the extension, then leading md directory, and then prepend leading rs directory.

However, there are also some files in the second hierarchy which are on its own, i.e. they do not have matching .md files.

The program should recursively look at the .rs files, try to match them with .md files, and output the following statistics:

1) total byte and line count values for all the files in the first hierarchy and the matching files in the second hierarchy - to evaluate how big the difference is.
2) byte and line count values for the .rs files which do not have the matching .md files - to have an idea which files do not have a source template
3) for the matching files - per-pair byte and line counts from each of the files for comparison.

For the implementation: Path does not have push() and pop() methods.

Do not use any markdown separators please.

Provides clear error messages and status updates.

Only updates the file when the new implementation is deemed better.

