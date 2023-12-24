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

use crate::{
    drivers::{
        disk::ata::{self, read_from_disk, write_to_disk, Channel, DiskLoc, Drive},
        fs::{
            fs_driver,
            fs::{parse_sectors, FilePath, Fat32Element},
        },
    },
    print, println,
    prompt::{input, COMMANDS_HISTORY, COMMANDS_INDEX},
    serial_print, serial_println,
    state::{fs_driver, get_state},
};

use super::console::{ScreenChar, DEFAULT_CHAR};
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

    unsafe { crate::writer::outb(port, data) };
    Ok(())
}
// BOTH ARE UNSAFE BUT ITS FOR EASIER CODE
fn inb(args: I) -> O {
    let mut args = args.split(" ");
    let port = args.next().ok_or("Invalid argument: missing port")?;
    let port = port
        .parse()
        .map_err(|e| format!("Failed to parse port: {}", e))?;

    println!("{}", unsafe { crate::writer::inb(port) });
    Ok(())
}
fn read(raw_args: I) -> O {
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
    
    
    let sectors = read_from_disk(DiskLoc(channel, drive), start, end)?;
    let sectors = 
    if raw_args.contains("num") {
        let mut nums = String::new();
        for n in sectors {
            nums.push_str(&format!("{}, ", n));
        }
        nums
    } else {
        parse_sectors(&sectors)
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
fn write(raw_args: I) -> O {
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
    let mut i = 0;
    for word in content.iter() {
        for c in word.chars() {
            bytes.push(c as u8);
            i += 1;
        }
    }
    let sectors = write_to_disk(DiskLoc(channel, drive), start, bytes)?;
    println!("Done");
    Ok(())
}
// fn install(args:I) -> O{
//     let mut args = args.split(" ");
//     let channel = match args.next().ok_or("Invalid argument: missing channel (Primary/0, Secondary/1)")? {
//         "Primary" => Channel::Primary,
//         "0" => Channel::Primary,
//         "Secondary" => Channel::Secondary,
//         "1" => Channel::Secondary,
//         _ => return Err("Wrong channel: Primary//0 or Secondary//1".to_string()),
//     };
//     let drive = match args.next().ok_or("Invalid argument: missing drive (Master/0, Slave/1)")? {
//         "Master" => Drive::Master,
//         "0" => Drive::Master,
//         "Slave" => Drive::Slave,
//         "1" => Drive::Slave,
//         _ => return Err("Wrong drive: Master//0 or Slave//1".to_string()),
//     };
//     let loc = DiskLoc(channel, drive);
//     println!("Installing os on drive {:?} with channel {:?}", drive, channel);
//     let sector = Vec::new();
//     let bytes_per_sector: u16 = 5;
//     // [0xEB, 0x3C, 0x90,
//     // 'M' as u8, 'S' as u8, 'W' as u8, 'I' as u8, 'N' as u8, '4' as u8, '.' as u8, '1' as u8,
//     // (bytes_per_sector & 0xFF) as u8, (bytes_per_sector >> 8) as u8,
//     // sectors_per_cluster,
//     // reserved_sector,
//     // n_fats,
//     // (root_dir_entries & 0xFF) as u8, (root_dir_entries >> 8) as u8,
//     // (sectors & 0xFF) as u8, (sectors >> 8) as u8,];
//     write_to_disk(loc, 0, &sector[..]);
//     Ok(())
// }

fn read_file(raw_args: I) -> O {
    let mut args = raw_args.split(" ");
    let file_name = args.next().unwrap();
    if let Some(content) = get_state().fs().lock().read_file(&FilePath{raw_path:file_name.to_string()}) {
        println!("{}", content);
    } else {
        println!("Specified path couldn't be found")
    }
    Ok(())
}
fn read_dir(raw_args: I) -> O {
    let mut args = raw_args.split(" ");
    let file_name = args.next().unwrap();
    if let Some(content) = get_state().fs().lock().read_dir(&FilePath{raw_path:file_name.to_string()}) {
        for (path, ele) in content.iter() {
            println!("- {:?} -> {:?}", path.raw_path, ele);
        }
    } else {
        println!("Specified path couldn't be found")
    }
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
        for b in read_from_disk(DiskLoc(channel, drive), i, 3).unwrap() {
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
            f( "read",  "Read raw data from a disk", read),
            f( "write",  "Writes raw data to a disk", write),
            f( "clear", "Clears screen", |v:I| -> O {
                crate::terminal::console::clear_console();
                Ok(())
            }),
            f( "read_file", "Reads a file from fat disk", read_file),
            f( "read_dir",  "Reads a dir from fat disk",  read_dir),
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
                if history_len>1{
                    if COMMANDS_HISTORY.read().get(history_len-2).unwrap().len()==0 {
                        COMMANDS_HISTORY.write().swap(history_len-2, history_len-1);
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
