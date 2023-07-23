use alloc::{vec::Vec, string::{String, ToString}, boxed::Box, sync::Arc};
use hashbrown::HashMap;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::{prompt::input, println, print};
type Commands = HashMap<String, (Arc<dyn Fn(String) + Send + Sync>, String)>;
// fn f(fu: &(dyn Fn(String) + Send + Sync + 'static)) -> impl Fn(String) + Send + Sync {
//     fu
// }

lazy_static! {
    pub static ref SHELL_COMMANDS: Commands = {
        let mut c: Commands = HashMap::new();
        #[allow(non_snake_case)] // If const need to provide type which mean
        let CONSTANT_COMMANDS = [//TODO Make this cleaner
            ("echo",
            (Arc::new(|v| print!("{}", v)) as Arc<dyn Fn(String) + Send + Sync>,                "Prints args to console")),
            ("ls",
            (Arc::new(|v| print!("Doesn't support fs !")) as Arc<dyn Fn(String) + Send + Sync>, "Prints files | NOT SUPPORTED")),
        ];
        for (prog, (fun,desc)) in CONSTANT_COMMANDS {
            c.insert(prog.to_string(), (fun, desc.to_string()));
        }
        c
    };
}

pub struct CommandRunner {
    previous: Vec<String>,
    prefix: String,
    commands: Commands
}
impl CommandRunner {
    pub fn new(prefix: &str, commands:Commands) -> Self {
        Self {
            previous: Vec::new(),
            prefix: String::from(prefix),
            commands,
        }
    }
    pub fn print_help(&mut self) { //TODO Make it so we don't need &mut because we have to add to self.previous
        println!("Available commands:");
        for (command, (fun, description)) in self.commands.iter() {
            println!("- {} -> {}", command, description);
        }
    }
    pub fn run(mut self) {
        'commands: loop{
            let b = input(&self.prefix); // Binding for longer lived value
            let mut c = b.split(" ");
            let program = c.next().unwrap(); //TODO Crash if user types nothing, handle error
            if let Some((fun,desc)) = self.commands.get(program) {
                fun(c.into_iter().map(|s| alloc::string::ToString::to_string(&s)).collect::<Vec<String>>().join(" "));
            } else {
                print!("\nUnsupported command, mispelled ? These are the "); self.print_help()
            }
            self.previous.push(b);
        }
    }
}
pub struct Shell {
    inner: CommandRunner
}
impl Shell {
    pub fn new() -> () {
        Self {
            inner: CommandRunner::new("> ", SHELL_COMMANDS.clone())
        }.inner.run()
    }
}