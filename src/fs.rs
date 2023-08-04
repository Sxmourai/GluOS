use alloc::string::{String, ToString};


pub enum OpenError {
    NotFound, 
    PermissionDenied, // Shouldn't happen... But keep this for rlib ?
}
pub enum ReadError {
    NotFound, // This could happen with multithreading
    //TODO Find a way to make sure that when a file is opened, it cannot be deleted
    PermissionDenied, 
}
pub enum WriteError {
    NotFound,
    PermissionDenied,
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
    pub fn read(&self) -> Result<String, ReadError> {
        Ok(todo!("Read"))
    }
    pub fn write(&self, content: String) -> Result<(), WriteError> {
        Ok(())
    }
}

pub fn open(filename: &str) -> Result<File, OpenError> {
    Ok(File::new(filename.to_string()))
}
pub fn read(filename: &str) -> Result<String, ReadError> {
    open(filename)?.read()
}
pub fn write(filename: &str, content: &str) -> Result<(), WriteError> {
    open(filename).write(content.to_string())
}
pub fn delete(filename: &str) -> Result<(), WriteError> {
    open(filename).delete()
}