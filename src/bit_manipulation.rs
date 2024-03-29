use core::{
    fmt::Display,
    mem::size_of,
    ops::{BitOr, Shl},
};

use alloc::string::String;
use x86_64::structures::port::{PortRead, PortWrite};

use crate::serial_print;

pub fn serial_print_all_bits<T: Into<u128>>(num: T) {
    let num = num.into();
    let size = core::mem::size_of::<T>() * 8;

    for i in (0..size).rev() {
        let bit = (num >> i) & 1;
        serial_print!("{}", bit);
    }
    serial_print!(" - ");
}

pub fn bytes<T: Into<u128>>(num: T) -> String {
    let mut result = String::new();
    let num = num.into();
    let size = core::mem::size_of::<T>() * 8;

    for i in (0..size).rev() {
        let bit = (num >> i) & 1;
        let str_bit = if bit == 0 { '0' } else { '1' };
        result.push(str_bit);
    }
    result
}
pub fn bytes_list<T: Into<u128> + Copy, I: IntoIterator<Item = T>>(list: I) -> String {
    let mut result = String::new();
    for ele in list {
        result.push_str(bytes(ele).as_str());
    }
    result
}
pub fn numeric_to_char_vec<T>(value: T) -> String
where
    T: Into<u64>,
{
    let value_u64 = value.into();
    let mut char_vec = String::new();

    for shift in (0..(core::mem::size_of::<T>() * 8)).step_by(8) {
        let byte: u8 = ((value_u64 >> shift) & 0xFF) as u8;
        char_vec.push(char::from(byte));
    }

    char_vec
}
#[must_use] pub fn bytes_to_numeric<T>(bytes: &[u8]) -> T
where
    T: From<u64>,
{
    let mut result: u64 = 0;

    for (i, &byte) in bytes.iter().enumerate() {
        if i < core::mem::size_of::<T>() {
            result |= u64::from(byte) << (i * 8);
        }
    }

    T::from(result)
}
pub fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    unsafe { return core::slice::from_raw_parts((p as *const T).cast::<u8>(), core::mem::size_of::<T>()) }
}
// mut iter: impl Iterator<Item=impl PartialEq<u8>>
#[must_use] pub fn all_zeroes(mut iter: &[u8]) -> bool {
    iter.iter().all(|x| *x == 0)
}

pub fn list_to_num<T, R>(content: impl Iterator<Item = T> + DoubleEndedIterator) -> R
where
    T: Into<R>,
    R: BitOr<Output = R> + Shl<usize> + From<<R as Shl<usize>>::Output> + Default,
{
    let mut result = R::default();
    for (i, byte) in content.into_iter().rev().enumerate() {
        if i >= size_of::<R>() / size_of::<T>() {
            break;
        }
        result = Into::<R>::into(result << (size_of::<T>() * 8)) | byte.into();
    }
    result
}
pub fn ptrlist_to_num<'a, T, R>(
    content: &mut (impl Iterator<Item = &'a T> + ?Sized + DoubleEndedIterator),
) -> R
where
    T: Into<R> + 'a + Clone,
    R: BitOr<Output = R> + Shl<usize> + From<<R as Shl<usize>>::Output> + Default,
{
    let mut result = R::default();
    for (i, byte) in content.into_iter().rev().enumerate() {
        if i >= size_of::<R>() / size_of::<T>() {
            break;
        }
        result = Into::<R>::into(result << (size_of::<T>() * 8)) | Into::<R>::into(byte.clone());
    }
    result
}

#[must_use] pub fn u16_to_u8(w: u16) -> (u8, u8) {
    (((w >> 8) as u8), (w & 0xFF) as u8)
}
struct CharArray<const N: usize>([char; N]);

impl<const N: usize> Display for CharArray<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut s = String::new();
        for element in &self.0 {
            s.push(*element);
        }
        write!(f, "{s}")
    }
}
impl<const N: usize> core::fmt::Debug for CharArray<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut s = String::new();
        for element in &self.0 {
            s.push(*element);
        }
        write!(f, "[{s}]")
    }
}
struct CharSlice([char]);

impl Display for CharSlice {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut s = String::new();
        for element in &self.0 {
            s.push(*element);
        }
        write!(f, "{s}")
    }
}
impl core::fmt::Debug for CharSlice {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut s = String::new();
        for element in &self.0 {
            s.push(*element);
        }
        write!(f, "[{s}]")
    }
}
struct CharSlicePtr<'a>(&'a [char]);

impl Display for CharSlicePtr<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut s = String::new();
        for element in self.0 {
            s.push(*element);
        }
        write!(f, "[{s}]")
    }
}

#[must_use] pub fn as_chars(list: &[u8]) -> String {
    let mut r = String::new();
    for byte in list {
        r.push(*byte as char);
    }
    r
}
/// # Safety
/// Ensure that writing to this port doesn't crash anything
pub fn outb(port: u16, val: u8) {
    unsafe { PortWrite::write_to_port(port, val) }
}
/// # Safety
/// Ensure that reading to this port doesn't crash anything
#[must_use] pub fn inb(port: u16) -> u8 {
    unsafe { PortRead::read_from_port(port) }
}

// pub fn is_set<T: core::ops::BitAnd<usize>>(value: T,n: usize) -> bool {
//     Into::<<T as core::ops::BitAnd<usize>>::Output>::into(value & n) != 0
// }

/// # Safety
/// Breaks memory safety from rust
#[must_use] pub unsafe fn read_at<T: Clone>(addr: usize) -> T {
    unsafe { (*(addr as *const T)).clone() }
    // todo!()
}

/// # Safety
/// Breaks memory safety from rust
pub unsafe fn write_at<T: Clone>(addr: usize, val: T) {
    unsafe { *(addr as *mut T) = val }
}
