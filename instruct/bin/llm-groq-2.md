Please write a Rust program that implements calling Groq model. Do not stop the output until you output the whole program. The code MUST compile from the first shot.

Do not use any markdown separators please.

I would like you to have the code accept two mandatory arguments being input and output files names, 
and the code should do the following with them:

- if the output file is non-existent or empty, it should just feed the contents of the input file after the following preamble: 

   "Please produce single output result, which would match the description below as well as you can:"; 

- if the file exists, then the prompt needs to be different:

   "Please verify that the description below (enclosed into <result-description></result-description>) matches the specimen (enclosed into <result-specimen></result-specimen>) as much as possible. If it does - then simply output the content of the result-specimen verbatim. If you find that there are imperfections in how result-specimen fulfills its purpose described in result-description, then improve it and output the full result, with your improvements. Do not delimit the result with anything, output it verbatim." 

Save the entire request into a file "/tmp/llm-req-<pid>-gen.txt" for reference.

Get the response from LLM, save it in its entirety into "/tmp/llm-req-<pid>-gen-resp.txt".

Perform "cargo check" with necessary flags to obtain json output, and filter the error messages only, that relate to the file in question.

Rename the original output file with the name ".orig" appended to it, and save the copy of new LLM reply into output file.

Perform "cargo check" with necessary flags to obtain json output *AGAIN*, and filter the error messages only, that relate to the file in question.

After obtaining the reply from LLM, and before rewriting the output_result file, it should submit another request with the following prompt: "Please CAREFULLY evaluate the below description (enclosed into <result-description></result-description>), and two outputs corresponding to this description, first one enclosed into "<first-result></first-result>" and the second enclosed into "<second-result></second-result>", with compile errors of first result included into "<first-compile-errors></first-compile-errors>" and second compile errors as "<second-compile-errors></second-compile-errors>", and evaluate which of the two is more precise and correct in implementing the description - and also which of them compiles! Then, if the first result is better, output the phrase 'First result is better.', if the second result is better, output the phrase 'The second implementation is better.'. Output only one of the two phrases, and nothing else"

Then, include the contents of the file with the description (first program argument is the file name), the original content (file name is the second argument), and the content of the first LLM response, and the subset of the compiler errors.

Save the entire request into a file "/tmp/llm-req-<pid>-eval.txt" for reference

Get the response from LLM, save it in its entirety into "/tmp/llm-req-<pid>-eval-resp.txt" and check its contents.

If the response is "The second implementation is better." then the program would write the content of the output of the model into the output file name.

If the response is "First result is better." then if the compiler errors output is empty, then restore the original file from .orig, and just update the mtime attribute on the file so it is seen as modified by an underlying OS.

If the response is anything else then exit with an error.

Important: do not delete the draft file, unless you have used the result from it - keep it for diagnostic purposes in case of error or bad suggestions - however, if its contents are not accepted, rename it with ".rej" instead of ".draft".

Use lower temperature (0.1) and fewer max_tokens (100) for the evaluation call to get more consistent responses.

Only updates the file when the new implementation is deemed better.

# Implementation details

For groq interaction, do not create new code, but rather use a pre-existing library, which you can use by adding "mod lib;" into your code - this will refer to a preexisting library inside the source tree.

Then, "lib::groq::Groq::new()" will return you a new instance of Groq, and calling ".evaluate(prompt)"
on that instance will return you the evaluated response.

The program must be in a simple sync fashion, do not use async please.

Before each action - e.g. making LLM requests, renaming files, calling cargo check, etc. - send a short status message to stderr.

