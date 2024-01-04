use core::{fmt::Display, mem::size_of, ops::{Shl, BitOr}};

use alloc::string::String;

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
        let str_bit = if bit == 0 {'0'} else {'1'};
        result.push(str_bit);
    }
    result
}
pub fn bytes_list<T: Into<u128> + Copy, I: IntoIterator<Item = T>>(list: I) -> String {
    let mut result = String::new();
    for ele in list.into_iter() {
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
pub fn bytes_to_numeric<T>(bytes: &[u8]) -> T
where
    T: From<u64>,
{
    let mut result: u64 = 0;

    for (i, &byte) in bytes.iter().enumerate() {
        if i < core::mem::size_of::<T>() {
            result |= (byte as u64) << (i * 8);
        }
    }

    T::from(result)
}
pub fn slice16_to_str(slice: &[u16]) -> String {    // return String::from_utf16_lossy(slice);
    let mut content = String::new();
    for (i,w) in slice.iter().enumerate() {
        content.push(((w & 0xFF) as u8) as char);// Interpreted as chars
        content.push(((w >> 8) as u8) as char); //Transforms the word into two bytes
    }
    content
}
pub unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    unsafe{core::slice::from_raw_parts((p as *const T) as *const u8, core::mem::size_of::<T>())}
}

///! DANGER ZONE DONT GO THERE 🤣
pub fn list_to_num<T,R>(mut content: impl Iterator<Item = T> + DoubleEndedIterator) -> R 
where T: Into<R>,
      R: BitOr<Output = R> + Shl<usize> + From<<R as Shl<usize>>::Output> + Default{
  let mut result = R::default();
  for (i, byte) in content.into_iter().rev().enumerate() {
      if i >= size_of::<R>()/size_of::<T>() {break}
      result = Into::<R>::into((result << size_of::<T>()*8)) | byte.into();
  }
  result
}
pub fn ptrlist_to_num<'a, T,R>(mut content: &mut (impl Iterator<Item = &'a T> + ?Sized + DoubleEndedIterator)) -> R 
where T: Into<R> + 'a + Clone,
      R: BitOr<Output = R> + Shl<usize> + From<<R as Shl<usize>>::Output> + Default{
  let mut result = R::default();
  for (i, byte) in content.into_iter().rev().enumerate() {
      if i >= size_of::<R>()/size_of::<T>() {break}
      result = Into::<R>::into((result << size_of::<T>()*8)) | Into::<R>::into(byte.clone());
  }
  result
}

pub fn u16_to_u8(w: u16) -> (u8, u8) {
    (((w >> 8) as u8), (w & 0xFF) as u8)
}
struct CharArray<const N: usize> ([char; N]);

impl<const N: usize> Display for CharArray<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut s = String::new();
        for element in &self.0 {
            s.push(*element);
        }
        write!(f, "[{}]", s)
    }
}//TODO implement debugging
struct CharSlice ([char]);

impl Display for CharSlice {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut s = String::new();
        for element in &self.0 {
            s.push(*element);
        }
        write!(f, "[{}]", s)
    }
}//TODO implement debugging
struct CharSlicePtr<'a> (&'a [char]);

impl Display for CharSlicePtr<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut s = String::new();
        for element in self.0 {
            s.push(*element);
        }
        write!(f, "[{}]", s)
    }
}