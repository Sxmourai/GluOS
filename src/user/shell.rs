use core::fmt::Error;

use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    sync::Arc,
    vec::{self, Vec},
};
use hashbrown::HashMap;
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::structures::port::{PortRead, PortWrite};

use crate::{
    drivers::{
        disk::ata::{self, read_from_disk, write_to_disk, Channel, DiskLoc, Drive},
        fs::{fs::FilePath, fs_driver},
    },
    fs::fs::{Fat32Entry, ToFilePath},
    print, println, serial_print, serial_println,
    state::{fs_driver, get_state},
    terminal::console::{ScreenChar, DEFAULT_CHAR},
};

use super::prompt::{input, COMMANDS_HISTORY, COMMANDS_INDEX};
type Commands = HashMap<
    String,
    (
        Arc<dyn Fn(String) -> Result<(), String> + Send + Sync>,
        String,
    ),
>;

// Helper function to generate closures and convert them to Arc
fn f<F>(prog: &str, desc: &str, closure: F) -> (String, Arc<dyn Fn(I) -> O + Send + Sync>, String)
where
    F: Fn(I) -> O + Send + Sync + 'static,
{
    (
        prog.to_string(),
        Arc::new(closure) as Arc<dyn Fn(I) -> O + Send + Sync>,
        desc.to_string(),
    )
}
fn ls(args: I) -> O {
    for disk in &ata::disk_manager().as_ref().unwrap().disks {
        if let Some(disk) = disk.as_ref() {
            println!("-> {}", disk);
        }
    }
    Ok(())
}
// BOTH ARE UNSAFE BUT ITS FOR EASIER CODE
fn outb(args: I) -> O {
    let mut args = args.split(" ");
    let port = args.next().ok_or("Invalid argument: missing port")?;
    let data = args.next().ok_or("Invalid argument: missing data")?;
    let port = port
        .parse()
        .map_err(|e| format!("Failed to parse port: {}", e))?;
    let data = data
        .parse()
        .map_err(|e| format!("Failed to parse data: {}", e))?;

    unsafe { u8::write_to_port(port, data) };
    Ok(())
}
// BOTH ARE UNSAFE BUT ITS FOR EASIER CODE
fn inb(args: I) -> O {
    let mut args = args.split(" ");
    let port = args.next().ok_or("Invalid argument: missing port")?;
    let port = port
        .parse()
        .map_err(|e| format!("Failed to parse port: {}", e))?;

    println!("{}", unsafe { u8::read_from_port(port) });
    Ok(())
}
fn read_sector(raw_args: I) -> O {
    let mut args = raw_args.split(" ");
    let channel = match args
        .next()
        .ok_or("Invalid argument: missing channel (Primary/0, Secondary/1)")?
    {
        "Primary" => Channel::Primary,
        "0" => Channel::Primary,
        "Secondary" => Channel::Secondary,
        "1" => Channel::Secondary,
        _ => return Err("Wrong channel: Primary//0 or Secondary//1".to_string()),
    };
    let drive = match args
        .next()
        .ok_or("Invalid argument: missing drive (Master/0, Slave/1)")?
    {
        "Master" => Drive::Master,
        "0" => Drive::Master,
        "Slave" => Drive::Slave,
        "1" => Drive::Slave,
        _ => return Err("Wrong drive: Master//0 or Slave//1".to_string()),
    };
    let start = args
        .next()
        .ok_or("Invalid argument: missing start address (u64)")?;
    let end = args
        .next()
        .ok_or("Invalid argument: missing end address (u64)")?;
    let start = start
        .parse()
        .map_err(|e| format!("Failed to parse start: {}", e))?;
    let end = end
        .parse()
        .map_err(|e| format!("Failed to parse end: {}", e))?;

    let sectors = read_from_disk(&DiskLoc(channel, drive), start, end)?;
    let sectors = if raw_args.contains("num") {
        let mut nums = String::new();
        for n in sectors {
            nums.push_str(&format!("{}, ", n));
        }
        nums
    } else {
        String::from_utf8_lossy(&sectors).to_string()
    };
    if raw_args.contains("--serial") {
        serial_println!("{:#?}", sectors);
    }
    if raw_args.contains("raw") {
        println!("{:#?}", sectors);
    } else {
        println!("{}", sectors);
    }
    Ok(())
}
fn write_sector(raw_args: I) -> O {
    let mut args = raw_args.split(" ");
    let channel = match args
        .next()
        .ok_or("Invalid argument: missing channel (Primary/0, Secondary/1)")?
    {
        "Primary" => Channel::Primary,
        "0" => Channel::Primary,
        "Secondary" => Channel::Secondary,
        "1" => Channel::Secondary,
        _ => return Err("Wrong channel: Primary//0 or Secondary//1".to_string()),
    };
    let drive = match args
        .next()
        .ok_or("Invalid argument: missing drive (Master/0, Slave/1)")?
    {
        "Master" => Drive::Master,
        "0" => Drive::Master,
        "Slave" => Drive::Slave,
        "1" => Drive::Slave,
        _ => return Err("Wrong drive: Master//0 or Slave//1".to_string()),
    };
    let start = args
        .next()
        .ok_or("Invalid argument: missing start address (u64)")?;
    let content: Vec<&str> = args.collect();
    let start = start
        .parse()
        .map_err(|e| format!("Failed to parse start: {}", e))?;
    let mut bytes = Vec::new();
    for word in content.iter() {
        for c in word.chars() {
            bytes.push(c as u8);
        }
    }
    let sectors = write_to_disk(DiskLoc(channel, drive), start, bytes)?;
    println!("Done");
    Ok(())
}

fn read(raw_args: I) -> O {
    let mut args = raw_args.split(" ");
    let path = args.next().unwrap();
    let mut binding = get_state();
    let fs_driver = binding.fs().lock();
    if let Some(entry) = fs_driver.get_entry(&path.to_filepath()) {
        match entry {
            Fat32Entry::File(file) => {
                let content = fs_driver.read_file(&path.to_filepath());
                println!("{}", content.unwrap()); // Can safely unwrap because we know the file exists
            }
            Fat32Entry::Dir(dir) => {
                if let Some(entries) = fs_driver.read_dir_at_sector(&dir.path, dir.sector as u64)
                {
                    for (path, inner_entry) in entries.iter() {
                        let name = match inner_entry {
                            Fat32Entry::File(file) => ("File ", file.path(), file.size),
                            Fat32Entry::Dir(dir) => ("Dir ", dir.path(), dir.sector as u64),
                        };
                        println!("- {:?} -> {:?}", path.path(), name);
                    }
                }
            }
        }
    } else {
        println!("Specified path couldn't be found")
    }
    Ok(())
}

fn write(raw_args: I) -> O { // TODO Refactor input/output for PROPER error handling
    let mut args = raw_args.split(" ");
    let entry_type = args.next().unwrap();
    let path = args.next().unwrap();
    let mut binding = get_state();
    let mut fs_driver = binding.fs().lock();
    if let Some(entry) = fs_driver.get_entry(&path.to_filepath()) {
        println!("File already exists !");
        return Ok(())
    }
    let content = args.collect::<String>();
    match entry_type {
        "dir" => {
            if !content.is_empty() {
                println!("Useless to specify content, created a empty dir");
            }
            fs_driver.write_dir(path.to_filepath()).unwrap()
        },
        "file" => {
            if content.is_empty() {
                println!("Created a empty file");
                return Ok(())
            }
            fs_driver.write_file(path.to_filepath(), content).unwrap()
        },
        _ => {
            println!("Invalid entry type ! dir/file")
        }
    };
    Ok(())
}

pub fn dump_disk(args: I) -> O {
    let mut args = args.split(" ");
    let channel = match args
        .next()
        .ok_or("Invalid argument: missing channel (Primary/0, Secondary/1)")?
    {
        "Primary" => Channel::Primary,
        "0" => Channel::Primary,
        "Secondary" => Channel::Secondary,
        "1" => Channel::Secondary,
        _ => return Err("Wrong channel: Primary//0 or Secondary//1".to_string()),
    };
    let drive = match args
        .next()
        .ok_or("Invalid argument: missing drive (Master/0, Slave/1)")?
    {
        "Master" => Drive::Master,
        "0" => Drive::Master,
        "Slave" => Drive::Slave,
        "1" => Drive::Slave,
        _ => return Err("Wrong drive: Master//0 or Slave//1".to_string()),
    };
    let mut i = 0;
    loop {
        for b in read_from_disk(&DiskLoc(channel, drive), i, 3).unwrap() {
            if b != 0 {
                serial_print!("{b} ")
            }
        }
        i += 1;
    }

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
            f( "ls","Prints disks", ls),
            f( "outb",  "Send data (u8) to a port (u16)", outb),
            f( "inb",   "Read data (u8) from a port (u16) and prints it",inb),
            f( "read_raw",  "Read raw data from a disk", read_sector),
            f( "write_raw",  "Writes raw data to a disk", write_sector),
            f( "clear", "Clears screen", |v:I| -> O {
                crate::terminal::console::clear_console();
                Ok(())
            }),
            f( "read",  "Reads a entry from fat disk (If dir it's like 'ls' and if file is like 'cat'",  read),
            f( "write", "Writes a entry to fat disk (If dir just path and if file path+content",  write),
            f( "dump_disk", "Reads all disk into serial", dump_disk),
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
            let mut command = Vec::new();
            for char in b.bytes() {
                command.push(ScreenChar::new(char, DEFAULT_CHAR.color_code));
            }
            unsafe {
                unsafe { COMMANDS_HISTORY.write().push(command) };
                let history_len = COMMANDS_HISTORY.read().len();
                if history_len > 1 {
                    if COMMANDS_HISTORY.read().get(history_len - 2).unwrap().len() == 0 {
                        COMMANDS_HISTORY
                            .write()
                            .swap(history_len - 2, history_len - 1);
                    }
                }
            }
            unsafe { *COMMANDS_INDEX.write() += 1 };

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
