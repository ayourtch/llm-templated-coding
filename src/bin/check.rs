mod lib;

fn main() {
  println!("hello");

  let test = lib::ollama::OllamaClient::new("localhost");
  println!("Test result: {:?}", &test);
}
