#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]

#[cfg(feature = "axstd")]
use axstd::printlnc;

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    printlnc!("[WithColor]: Hello, Arceos!");
}
