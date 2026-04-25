#[allow(unused_imports)]

mod repl;
mod path;
mod tokenizer;
mod builtins;
mod exec;


fn main() {
    repl::start_shell_repl();
}
