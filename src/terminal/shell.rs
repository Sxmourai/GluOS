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

use crate::{print, println, prompt::input, drivers::disk::{ata::{Channel, Drive, read_from_disk, DiskLoc}, fs::parse_sectors}, serial_println};
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
fn read(raw_args:I) -> O{
    let mut args = raw_args.split(" ");
    let channel = match args.next().ok_or("Invalid argument: missing channel (Primary/0, Secondary/1)")? {
        "Primary" => Channel::Primary,
        "0" => Channel::Primary,
        "Secondary" => Channel::Secondary,
        "1" => Channel::Secondary,
        _ => return Err("Wrong channel: Primary//0 or Secondary//1".to_string()),
    };
    let drive = match args.next().ok_or("Invalid argument: missing drive (Master/0, Slave/1)")? {
        "Master" => Drive::Master,
        "0" => Drive::Master,
        "Slave" => Drive::Slave,
        "1" => Drive::Slave,
        _ => return Err("Wrong drive: Master//0 or Slave//1".to_string()),
    };
    let start = args.next().ok_or("Invalid argument: missing start address (u64)")?;
    let end = args.next().ok_or("Invalid argument: missing end address (u64)")?;
    let start = start.parse().map_err(|e| format!("Failed to parse start: {}", e))?;
    let end = end.parse().map_err(|e| format!("Failed to parse end: {}", e))?;


    let sectors = read_from_disk(DiskLoc(channel, drive), start, end)?;
    if raw_args.contains("--serial") {
        serial_println!("{:#?}", parse_sectors(&sectors));
    }
    println!("{:#?}", parse_sectors(&sectors));
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
            f( "read",  "Read raw data from a disk ()", read),
            f( "clear", "Clears screen", |v:I| -> O {
                crate::terminal::console::clear_console();
                Ok(())
            })
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
                let args = c
                .into_iter()
                .map(|s| alloc::string::ToString::to_string(&s))
                .collect::<Vec<String>>()
                .join(" ");
                if let Err(error_message) = fun(args) {
                    println!("Error: {}", error_message);
                }
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
