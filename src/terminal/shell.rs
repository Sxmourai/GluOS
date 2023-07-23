use core::fmt::Error;

use alloc::{
    boxed::Box,
    string::{String, ToString},
    sync::Arc,
    vec::Vec, format,
};
use hashbrown::HashMap;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::{print, println, prompt::input};
type Commands = HashMap<String, (Arc<dyn Fn(String) -> Result<(), String> + Send + Sync>, String)>;

// Helper function to generate closures and convert them to Arc
fn f<F>(prog: &str, desc: &str, closure: F) -> (String, Arc<dyn Fn(I) -> O + Send + Sync>, String) where
    F: Fn(I) -> O + Send + Sync + 'static,
{   (
        prog.to_string(),
        Arc::new(closure) as Arc<dyn Fn(I) -> O + Send + Sync>,
        desc.to_string(),
    )}

// BOTH ARE UNSAFE BUT ITS FOR EASIER CODE
fn outb(args:I) -> O{
    let mut args = args.split(" ");
    let port = args.next().ok_or("Invalid argument: missing port")?;
    let data = args.next().ok_or("Invalid argument: missing data")?;
    let port = port.parse().map_err(|e| format!("Failed to parse port: {}", e))?;
    let data = data.parse().map_err(|e| format!("Failed to parse data: {}", e))?;

    unsafe { crate::writer::outb(port, data) };
    Ok(())
}
// BOTH ARE UNSAFE BUT ITS FOR EASIER CODE
fn inb(args:I) -> O{
    let mut args = args.split(" ");
    let port = args.next().ok_or("Invalid argument: missing port")?;
    let port = port.parse().map_err(|e| format!("Failed to parse port: {}", e))?;

    println!("{}", unsafe{crate::writer::inb(port)});
    Ok(())
}
type I = String;
type O = Result<(), String>;
lazy_static! {
    pub static ref SHELL_COMMANDS: Commands = {
        let mut c: Commands = HashMap::new();
        #[allow(non_snake_case)] // If const need to provide type which mean
        let CONSTANT_COMMANDS = [
            f( "echo",  "Prints args to console",   
                |v:I| ->O {print!("{}", v);Ok(())}),

            f( "ls",    "Prints files | NOT SUPPORTED",
                |v:I| ->O {print!("Our kernel doesn't support fs !");Ok(())}, ),

            f( "outb",  "Send data (u8) to a port (u16)", outb),
            f( "inb",   "Read data (u8) from a port (u16) and prints it",inb),
        ];
        for (prog, fun, desc) in CONSTANT_COMMANDS {
            c.insert(prog, (fun, desc));
        }
        c
    };
}

pub struct CommandRunner {
    previous: Vec<String>,
    prefix: String,
    commands: Commands,
}
impl CommandRunner {
    pub fn new(prefix: &str, commands: Commands) -> Self {
        Self {
            previous: Vec::new(),
            prefix: String::from(prefix),
            commands,
        }
    }
    pub fn print_help(&mut self) {
        //TODO Make it so we don't need &mut because we have to add to self.previous
        println!("Available commands:");
        for (command, (fun, description)) in self.commands.iter() {
            println!("- {} -> {}", command, description);
        }
    }
    pub fn run(mut self) {
        'commands: loop {
            let b = input(&self.prefix); // Binding for longer lived value
            let mut c = b.split(" ");
            let program = c.next().unwrap(); //TODO Crash if user types nothing, handle error
            if let Some((fun, desc)) = self.commands.get(program) {
                fun(c
                    .into_iter()
                    .map(|s| alloc::string::ToString::to_string(&s))
                    .collect::<Vec<String>>()
                    .join(" "));
            } else {
                print!("\nUnsupported command, mispelled ? These are the ");
                self.print_help()
            }
            self.previous.push(b);
        }
    }
}
pub struct Shell {
    inner: CommandRunner,
}
impl Shell {
    pub fn new() -> () {
        Self {
            inner: CommandRunner::new("> ", SHELL_COMMANDS.clone()),
        }
        .inner
        .run()
    }
}
