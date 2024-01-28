use alloc::{
    string::{String, ToString},
    vec::Vec,
};


use super::path::FilePath;

#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub struct FatPermissions(pub u8);
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct FatGroup {
    pub group_name: String,
    pub id: u16,
    pub derived_groups: Vec<u16>,
}
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct FatUser {
    pub username: String,
    pub id: u16,
    pub groups: Vec<u16>,
}
pub fn get_group(id: u16) -> FatGroup {
    FatGroup {
        group_name: "default".to_string(),
        id,
        derived_groups: Vec::new(),
    }
}
pub fn get_user(id: u16) -> FatUser {
    FatUser {
        username: "Sxmourai".to_string(),
        id,
        groups: alloc::vec![1],
    }
}
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum FatPerson {
    Group(FatGroup),
    User(FatUser),
}
impl FatPerson {
    pub fn new(group: bool, id: u16) -> Self {
        if group {
            Self::Group(get_group(id))
        } else {
            Self::User(get_user(id))
        }
    }
}
#[derive(Default, Debug, Clone)] // Proper debug for attributes (flags in self.flags)
pub struct FatAttributes {
    // flags: u16,
    // permissions: HashMap<FatPerson, FatPermissions>,
}
impl FatAttributes {
    // pub fn permissions(&self, group: &FatPerson) -> Option<&FatPermissions> {
    //     self.permissions.get(group)
    // }
}
pub trait Fat32Element: core::fmt::Debug {
    fn path(&self) -> &FilePath;
    fn name(&self) -> &str;
    fn size(&self) -> u64;
    fn attributes(&self) -> &FatAttributes;
}
