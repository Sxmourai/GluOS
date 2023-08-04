use alloc::string::{String, ToString};


pub enum DiskError {
    NotFound, 
    PermissionDenied, // Shouldn't happen... But keep this for rlib ?
}

pub struct File {
    name: String
}
impl File {
    pub fn new(name:String) -> Self {
        Self {
            name,
        }
    }
    pub fn read(&self) -> Result<String, DiskError> {
        Ok(todo!("Read"))
    }
    pub fn write(&self, content: String) -> Result<(), DiskError> {
        Ok(())
    }
}

pub fn open(filename: &str) -> Result<File, DiskError> {
    Ok(File::new(filename.to_string()))
}
pub fn read(filename: &str) -> Result<String, DiskError> {
    open(filename)?.read()
}
pub fn write(filename: &str, content: &str) -> Result<(), DiskError> {
    open(filename).write(content.to_string())
}
pub fn delete(filename: &str) -> Result<(), DiskError> {
    open(filename).delete()
}