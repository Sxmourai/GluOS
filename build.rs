extern crate cmake;
use cmake::Config; 

fn main()
{
    let dst = Config::new("src/pci/c").build();

    println!("cargo:rustc-link-search=native={}", dst.display()); 
    println!("carog:rustc-link-lib=static=ide"); 
}