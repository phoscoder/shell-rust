#[allow(unused_imports)]

mod repl;
mod path;
mod tokenizer;
mod builtins;
mod exec;
mod completion_registry;


fn main() {
    repl::start_shell_repl();
}
