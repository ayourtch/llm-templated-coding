Please write a Rust program that implements calling gemini-2.5-pro using X-goog-api-key to supply a token from an environment variable GEMINI_API_KEY. Do not stop the output until you output the whole program. The code MUST compile from the first shot.

Do not use any markdown separators please.

I would like you to have the code accept two mandatory arguments being input and output files names, 
and the code should do the following with them:

- if the output file is non-existent or empty, it should just feed the contents of the input file after the following preamble: 

   "Please produce single output result, which would match the description below as well as you can:"; 

- if the file exists, then the prompt needs to be different:

   "Please verify that the description below (enclosed into <result-description></result-description>) matches the specimen (enclosed into <result-specimen></result-specimen>) as much as possible. If it does - then simply output the content of the result-specimen verbatim. If you find that there are imperfections in how result-specimen fulfills its purpose described in result-description, then improve it and output the full result, with your improvements. Do not delimit the result with anything, output it verbatim." 

Then the program would write the content of the output of the model into the output file name.

