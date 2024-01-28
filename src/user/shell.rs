

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use hashbrown::HashMap;

use raw_cpuid::CpuId;
use shell_macro::command;


use crate::{
    dbg, descriptor_tables, drivers::disk::ata::{self, read_from_disk, write_to_disk, Channel, DiskLoc, Drive}, fs_driver, print, println, serial_println, terminal::console::{ScreenChar, DEFAULT_CHAR}
};

use super::prompt::{input, COMMANDS_HISTORY, COMMANDS_INDEX};

#[command("lsdisk", "Lists plugged disks with size & slot")]
fn lsdisk(_args: String) -> Result<(), String> {
    #[cfg(feature="fs")]
    let drvs = fs_driver!();
    for (i, disk) in ata::disk_manager().as_ref().unwrap().disks.iter().enumerate() {
        if let Some(disk) = disk.as_ref() {
            println!("- {}", disk);
            #[cfg(feature="fs")]
            if let Some(partitions) = drvs.partitions.get(&disk.loc) {
                // If the partition start is 1 we know it's MBR because on GPT the first 33 sectors are reserved !
                let start_lba = partitions.get(0).and_then(|x| Some(x.1)).unwrap_or(0);
                if start_lba == 1 {
                    println!("--MBR--");
                }
                for part in partitions {
                    print!("|-> {}Kb ({} - {})",(part.2-part.1)/2, part.1, part.2);
                    if let Some(drv) = drvs.drivers.get(part) {
                        print!(" {}", drv.as_enum());
                    }
                    println!();
                }
                println!();
            }
        }
    }
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
    let _sectors = write_to_disk(DiskLoc(channel, drive), start, bytes)?;
    println!("Done");
    Ok(())
}

/// n = DriveLoc
/// p = Partition id
/// [n][p]/[path]
#[cfg(feature="fs")]
fn parse_path(path: &str) -> Option<crate::fs::fs::FilePath> {
    let loc_idx = path.chars().nth(0)?.to_string().parse::<u8>().ok()?;
    let loc = DiskLoc::from_idx(loc_idx)?;
    let part_idx = path.chars().nth(1)?.to_string().parse::<u8>().ok()?;
    let part = crate::fs::partition::Partition::from_idx(&loc, part_idx)?;
    Some(crate::fs::fs::FilePath::new(path[2..].to_string(), part.clone()))
}

// #[cfg(feature="fs")]
#[command("read", "Reads a file/dir from disk")]
fn read(raw_args: String) -> Result<(), String> {
    use crate::fs::fs_driver::Entry;
    let mut args = raw_args.split(" ");
    let path = parse_path(args.next().unwrap_or("0"));
    if path.is_none() {
        println!("Invalid path");
        return Ok(())
    }
    let path = path.unwrap();
    let fs_driver = fs_driver!();
    if let Ok(entry) = fs_driver.read(&path) {
        match entry {
            Entry::File(mut f) => {
                println!("{}",f.content);
            },
            Entry::Dir(mut d) => {
                for sub in d.entries {
                    println!("- {} ({}Kb)", sub.path, sub.size);
                }
            },
        }
    } else {
        println!("Error reading file ! Maybe specified path couldn't be found")
    }
    Ok(())
}

// #[command("write", "Writes a file to disk")]
// fn write(args: String) -> Result<(), String> {
//     // TODO Refactor input/output for PROPER error handling
//     let mut args = args.split(" ");
//     let entry_type = args.next().unwrap();
//     let path = parse_path(args.next().unwrap()).unwrap();
//     let mut fs_driver = unsafe{fs_driver!()};
//     if let Some(_entry) = fs_driver.get_entry(&path) {
//         println!("File already exists !");
//         return Ok(());
//     }
//     let content = args.collect::<String>();
//     match entry_type {
//         "dir" => {
//             if !content.is_empty() {
//                 println!("Useless to specify content, created a empty dir");
//             }
//             fs_driver.write_dir(path).unwrap()
//         }
//         "file" => {
//             if content.is_empty() {
//                 println!("Created a empty file");
//                 return Ok(());
//             }
//             fs_driver.write_file(path, content).unwrap()
//         }
//         _ => {
//             println!("Invalid entry type ! dir/file")
//         }
//     };
//     Ok(())
// }

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
    let loc = DiskLoc(channel, drive);
    loop {
        let sectors = read_from_disk(&loc, i, 1);
        if sectors.is_err() {break}
        let sectors = sectors.unwrap();
        if sectors.iter().all(|x| *x==0){
            if i%1000==0{
                serial_println!("-----------{}----------", i);
            }
        } else {
            serial_println!("\n\n-----------{}----------", i);
            serial_println!("{}", String::from_utf8_lossy(&sectors).to_string());
        }
        i += 1;
    }

    Ok(())
}

// #[command("test_disk", "Reads multiple times some sectors to see if same content is returned")]
// fn test_disk(args: String) -> Result<(), String> {
//     let mut args = args.split(" ");
//     let channel = match args
//         .next()
//         .ok_or("Invalid argument: missing channel (Primary/0, Secondary/1)")?
//     {
//         "Primary" => Channel::Primary,
//         "0" => Channel::Primary,
//         "Secondary" => Channel::Secondary,
//         "1" => Channel::Secondary,
//         _ => return Err("Wrong channel: Primary//0 or Secondary//1".to_string()),
//     };
//     let drive = match args
//         .next()
//         .ok_or("Invalid argument: missing drive (Master/0, Slave/1)")?
//     {
//         "Master" => Drive::Master,
//         "0" => Drive::Master,
//         "Slave" => Drive::Slave,
//         "1" => Drive::Slave,
//         _ => return Err("Wrong drive: Master//0 or Slave//1".to_string()),
//     };
//     let loc = DiskLoc(channel, drive);
//     let sector = read_from_disk(&loc, 5, 1).unwrap();
//     for i in 0..1_000 {
//         if read_from_disk(&loc, 5, 1).unwrap()!=sector {
//             println!("DISK / ATA CODE IS WRONG!");
//             return Ok(())
//         }
//     }
//     Ok(())
// }

#[cfg(feature="pci")]
#[command("lspci", "Lists pci devices connected to computer")]
fn lspci(args: String) -> Result<(), String> {
    let mut verbose = 0;
    if args.contains("-v") {
        verbose += 1;
    }
    for device in crate::pci::pci_device_iter() {
        if let Some(d) = pci_ids::Device::from_vid_pid(device.vendor_id, device.device_id) {
            let mut class = "Not found";
            let mut subclass = "Not found";
            for iter_class in pci_ids::Classes::iter() {
                if iter_class.id() == device.class {
                    for iter_subclass in iter_class.subclasses() {
                        if iter_subclass.id() == device.subclass {
                            //TODO Don't be afraid of nesting
                            class = iter_class.name();
                            subclass = iter_subclass.name();
                        }
                    }
                }
            }

            println!(
                "{}.{}.{} - {} {:?}",
                device.location.bus(),
                device.location.slot(),
                device.location.function(),
                d.name(),
                d.vendor().name(),
            );
            if verbose > 1 {
                println!(
                    "Class: {} - Subclass: {}\nSubsystems {:?}",
                    class, subclass, d,
                );
            }
        } else {
            crate::dbg!(device);
        }
    }

    Ok(())
}

#[command("sysinfo", "Gets info about computer")]
fn sysinfo(args: String) -> Result<(), String> {
    let mut ram_size = 0; //TODO Update bootloader, maybe we will be able to get mem size (cuz its a BIOS function)
    println!("RAM: {}", ram_size);
    let cpuid = CpuId::new();
    
    let vendor = match cpuid.get_vendor_info() {
        Some(vendor) => vendor.to_string(),
        None => "Unknown".to_string(),
    };
    let freq = match cpuid.get_processor_frequency_info() {
        Some(freq) => format!("Max: {} - Base: {}", freq.processor_max_frequency(), freq.processor_base_frequency()),
        None => "Unknown".to_string(),
    };
    let brand = match cpuid.get_processor_brand_string() {
        Some(brand) => brand.as_str().to_string(),
        None => "Unknown".to_string(),
    };
    let cores = unsafe{descriptor_tables!().num_core()};
    println!("CPU:\n- Vendor: {vendor}\n- Brand: {brand}\n- Frequency: {freq}\n- Cores: {cores}");
    if let Some(cparams) = cpuid.get_cache_parameters() {
        for cache in cparams {
            let size = cache.associativity() * cache.physical_line_partitions() * cache.coherency_line_size() * cache.sets();
            println!("- L{}-Cache size: {}", cache.level(), size);
        }
    } else {
        println!("- No cache parameter information available")
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
        for (
            name,
            Command {
                name: _,
                description,
                run: _,
            },
        ) in self.commands.iter()
        {
            println!("- {} -> {}", name, description);
        }
    }
    pub fn run(mut self) {
        loop {
            let cmd = input(&self.prefix); // Binding for longer lived value
            self.run_command(cmd)
        }
    }
    pub fn run_command(&mut self, cmd: String) {
        let mut command = Vec::new();
        for char in cmd.bytes() {
            command.push(ScreenChar::new(char, DEFAULT_CHAR.color_code));
        }
        unsafe {
            COMMANDS_HISTORY.write().push(command);
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

        let mut args = cmd.split(" ");
        let program = args.next().unwrap(); //TODO Crash if user types nothing, handle error
        if let Some(Command {
            name: _,
            description: _,
            run: fun,
        }) = self.commands.get(program)
        {
            let args = args
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
        self.previous.push(cmd);
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
    pub fn new() -> Self {
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
    }
    pub async fn run(self) {
        self.inner.run()
    }
    pub async fn run_with_command(mut self, cmd: String) {
        self.inner.run_command(cmd);
        self.inner.run()
    }
}
