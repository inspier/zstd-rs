#![cfg_attr(not(feature="std"), no_std)]

#[allow(unused_imports)]
#[cfg(feature = "std")]
use std as alloc;

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
use alloc::{
    boxed::Box,
    vec::Vec,
    string::String,
};

pub mod blocks;
pub mod io;
pub mod decoding;
pub mod errors;
pub mod frame;
pub mod frame_decoder;
pub mod fse;
pub mod huff0;
pub mod streaming_decoder;
mod tests;

pub const VERBOSE: bool = false;
pub use frame_decoder::BlockDecodingStrategy;
pub use frame_decoder::FrameDecoder;
pub use streaming_decoder::StreamingDecoder;
