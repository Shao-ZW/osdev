#![no_std]

pub mod result;
pub mod signal;

#[cfg(target_arch = "x86_64")]
pub mod x86_64;
