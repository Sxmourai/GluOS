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
use shell_macro::command;
use spin::Mutex;
use x86_64::structures::port::{PortRead, PortWrite};

use crate::{
    drivers::{
        disk::ata::{self, read_from_disk, write_to_disk, Channel, DiskLoc, Drive},
        fs::{fs::FilePath, fs_driver},
    },
    fs::fs::Fat32Entry,
    print, println, serial_print, serial_println,
    state::{fs_driver, get_state},
    terminal::console::{ScreenChar, DEFAULT_CHAR},
};

use super::prompt::{input, COMMANDS_HISTORY, COMMANDS_INDEX};

#[command("lsdisk", "Lists plugged disks with size & slot")]
fn lsdisk(args: String) -> Result<(), String> {
    for disk in &ata::disk_manager().as_ref().unwrap().disks {
        if let Some(disk) = disk.as_ref() {
            println!("-> {}", disk);
        }
    }
    Ok(())
}
fn outb(args: String) -> Result<(), String> {
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
fn inb(args: String) -> Result<(), String> {
    let mut args = args.split(" ");
    let port = args.next().ok_or("Invalid argument: missing port")?;
    let port = port
        .parse()
        .map_err(|e| format!("Failed to parse port: {}", e))?;

    println!("{}", unsafe { u8::read_from_port(port) });
    Ok(())
}

#[command("read_raw", "Reads a raw sector from disk")]
fn read_sector(raw_args: String) -> Result<(), String> {
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

#[command("write_sector", "Writes a raw sector to disk")]
fn write_sector(raw_args: String) -> Result<(), String> {
    let mut args = raw_args.split(" ");
    let channel = match args
        .next()
        .ok_or_else(|| "Invalid argument: missing channel (Primary/0, Secondary/1)")?
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

#[command("read", "Reads a file/dir from disk")]
fn read(raw_args: String) -> Result<(), String> {
    let mut args = raw_args.split(" ");
    let path = args.next().unwrap();
    let mut binding = get_state();
    let fs_driver = binding.fs().lock();
    if let Some(entry) = fs_driver.get_entry(&path.into()) {
        match entry {
            Fat32Entry::File(file) => {
                let content = fs_driver.read_file(&path.into());
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


#[command("write", "Writes a file to disk")]
fn write(args: String) -> Result<(), String> { // TODO Refactor input/output for PROPER error handling
    let mut args = args.split(" ");
    let entry_type = args.next().unwrap();
    let path = args.next().unwrap();
    let mut binding = get_state();
    let mut fs_driver = binding.fs().lock();
    if let Some(entry) = fs_driver.get_entry(&path.into()) {
        println!("File already exists !");
        return Ok(())
    }
    let content = args.collect::<String>();
    match entry_type {
        "dir" => {
            if !content.is_empty() {
                println!("Useless to specify content, created a empty dir");
            }
            fs_driver.write_dir(path).unwrap()
        },
        "file" => {
            if content.is_empty() {
                println!("Created a empty file");
                return Ok(())
            }
            fs_driver.write_file(path, content).unwrap()
        },
        _ => {
            println!("Invalid entry type ! dir/file")
        }
    };
    Ok(())
}

#[command("dump_disk", "Dumps disk to serial output (QEMU ONLY)")]
fn dump_disk(args: String) -> Result<(), String> {
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
        serial_println!("\n\n-----------{}----------", i);
        for b in read_from_disk(&DiskLoc(channel, drive), i, 3).unwrap() {
            if b != 0 {
                serial_print!("{}", b as char)
            }
        }
        i += 1;
    }

    Ok(())
}

#[command("lspci", "Lists pci devices connected to computer")]
fn lspci(args: String) -> Result<(), String> {
    for device in crate::pci::pci_device_iter() {
        let mut name;
        let mut subs;
        let mut vendor;
        let mut class = "Not found";
        let mut subclass = "Not found";
        if device.vendor_id == 0x1234 && device.device_id == 0x1111 { //TODO This is a workaround because pci_ids is not updated
            name = "QEMU Virtual Video Controller";
            vendor = "QEMU"; // Any Some other hypervisors use this device
            subs = Vec::new();
        } else {
            let d = pci_ids::Device::from_vid_pid(device.vendor_id, device.device_id).expect(&alloc::format!("Not found, {:?}", device));
            name = d.name();
            subs = d.subsystems().collect();
            vendor = d.vendor().name(); 
            
            for iter_class in pci_ids::Classes::iter() {
                if iter_class.id() == device.class {
                    for iter_subclass in iter_class.subclasses() {
                        if iter_subclass.id() == device.subclass { //TODO Don't be afraid of nesting
                            class = iter_class.name();
                            subclass = iter_subclass.name();
                        }
                    }
                }
            }
        }
        
        subs[0].name();
        serial_println!(
            "BUS: {}\t- {}\t-\tVendor {:?}\nClass: {}\t-\tSubclass: {}\nSubsystems {:?}\n\n",
            device.bus(),
            name,
            vendor,
            class,
            subclass,
            subs,
        );
    }

    Ok(())
}

pub struct CommandRunner {
    previous: Vec<String>,
    prefix: String,
    commands: HashMap<String, Command>,
}
impl CommandRunner {
    pub fn new(prefix: &str, commands: HashMap<String, Command>) -> Self {
        Self {
            previous: Vec::new(),
            prefix: String::from(prefix),
            commands,
        }
    }
    pub fn print_help(&mut self) {
        //TODO Make it so we don't need &mut because we have to add to self.previous
        println!("Available commands:");
        for (name, Command {name: _, description, run: _ }) in self.commands.iter() {
            println!("- {} -> {}", name, description);
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
            if let Some(Command {name, description, run: fun}) = self.commands.get(program) {
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
#[derive(Debug, Clone)]
pub struct Command {
    name: &'static str,
    description: &'static str,
    run: fn(String) -> Result<(), String>,
}

impl Shell {
    pub fn new() -> () {
        let commands = {
            let commands = shell_macro::command_list!();
            let mut res = HashMap::new();
            for command in commands {
                res.insert(command.name.to_string(), command.clone());
            }
            res
        };
        Self {
            inner: CommandRunner::new("> ", commands),
        }
        .inner
        .run()
    }
}
