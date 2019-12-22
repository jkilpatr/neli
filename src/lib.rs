//! # Type safety for the weary netlink user
//!
//! ## Rationale
//!
//! This crate aims to be a pure Rust implementation that defines
//! the necessary constants and wraps them in enums to distinguish between various categories of
//! constants in the context of netlink.
//!
//! ## The project is broken down into the following modules:
//! * `consts` - This is where all of the C-defined constants are wrapped into type safe enums for
//! use in the library.
//! * `err` - This module contains all of the protocol and library-level errors encountered in the
//! code.
//! * `genl` - This code provides parsing for the generic netlink subsystem of the netlink
//! protocol.
//! * `netfilter` - Netfilter related protocols (NFLOG, NFQUEUE, CONNTRACK).
//! * `nlattr` - This code provides more granular parsing methods for the generic netlink
//! attributes in the context of generic netlink requests and responses.
//! * `nl` - This is the top level netlink header code that handles the header that all netlink
//! messages are encapsulated in.
//! * `rtnl` - This module is for the routing netlink subsystem of the netlink protocol.
//! * `socket` - This provides a socket structure for use in sending and receiving messages and a
//! number of convenience functions for commonly encountered use cases.
//!
//! ## Traits
//!
//! The library at the top level contains the `Nl` trait which provides a buffer size calculation
//! function, a serialization method, and a deserialization method. It also contains
//! implementations of `Nl` for common types. The is one additional trait, `NlBuf`, used in cases
//! where, to deserialize a type, a buffer needs to be provided by the caller function and passed
//! to the callee.
//!
//! ## Design decisions
//!
//! This is a fairly low level library that currently does not have a whole lot of higher level
//! handle-type data structures and relies mostly on the `NlSocket` struct to provide most of the
//! convenience functions. I hope to add a higher level API by `v0.5.0` to ease some of the
//! workflows that have been brought to my attention.
//!
//! The goal of this library is completeness for handling netlink and am working to incorporate
//! features that will make this library easier to use in all use cases. If you have a use case you
//! would like to see supported, please open an issue on github.
//!
//! ## Examples
//!
//! Examples of working code exist in the `examples/` subdirectory on Github. They have a separate
//! `Cargo.toml` file to provide easy testing and use.
//!
//! ## Documentation
//!
//! Each module has been documented extensively to provide information on how to use the code
//! contained in the module. Pull requests for documentation mistakes, updates, and rewording for
//! clarity is a valuable contribution as this project aims to be as simple to use as
//! possible.

#![deny(missing_docs)]

#[cfg(feature = "async")]
extern crate mio;
#[cfg(feature = "async")]
extern crate tokio;

#[macro_use]
mod macros;

/// C constants defined as types
pub mod consts;
/// Error module
pub mod err;
/// Genetlink (generic netlink) header and attribute helpers
//pub mod genl;
mod neli_constants;
/// Nflog protocol (logging for netfilter)
//pub mod netfilter;
/// Top-level netlink header
pub mod nl;
/// Netlink attribute handler
//pub mod nlattr;
/// Route netlink bindings
//pub mod rtnl;
/// Wrapper for `libc` sockets
//pub mod socket;
/// Module for high level stream interface
//pub mod stream;
mod utils;

use std::{
    io::Write,
    mem, str,
};

use byteorder::ByteOrder;

pub use bytes::{Bytes, BytesMut};

use crate::{
    consts::alignto,
    err::{DeError, SerError},
};
pub use crate::{
    neli_constants::MAX_NL_LENGTH,
    utils::{U32BitFlag, U32Bitmask},
};

/// Trait defining basic actions required for netlink communication.
/// Implementations for basic and `neli`'s types are provided (see below). Create new
/// implementations if you have to work with a Netlink API that uses
/// values of more unusual types.
pub trait Nl: Sized {
    /// Serialization method
    fn serialize(&self, m: BytesMut) -> Result<BytesMut, SerError>;

    /// Deserialization method
    fn deserialize(m: Bytes) -> Result<Self, DeError>;

    /// The size of the binary representation of a type not aligned to work size
    fn type_size() -> Option<usize>;

    /// The size of the binary representation of a type not aligned to work size
    fn type_asize() -> Option<usize> {
        Self::type_size().map(alignto)
    }

    /// The size of the binary representation of an existing value not aligned to word size
    fn size(&self) -> usize;

    /// The size of the binary representation of an existing value aligned to word size
    fn asize(&self) -> usize {
        alignto(self.size())
    }

    /// Pad the data serialized data structure to alignment
    fn pad(&self, mut mem: BytesMut) -> Result<BytesMut, SerError> {
        let padding_len = self.asize() - self.size();
        if let Err(e) = mem.as_mut().write_all(&[0; libc::NLA_ALIGNTO as usize][..padding_len]) {
            Err(SerError::IOError(e, mem))
        } else {
            Ok(mem)
        }
    }
}

/// `Nl::deserialize()` alternative with lifetimes.
pub trait NlSlice<'a>: Sized + Nl {
    /// Deserialization method with byte slice
    fn deserialize_from_slice(m: &'a [u8]) -> Result<Self, DeError> {
        Self::deserialize(Bytes::from(m))
    }
}

impl Nl for u8 {
    fn serialize(&self, mut mem: BytesMut) -> Result<BytesMut, SerError> {
        let size = self.size();
        if mem.len() < size {
            return Err(SerError::UnexpectedEOB(mem));
        } else if mem.len() > size {
            return Err(SerError::BufferNotFilled(mem));
        }
        let _ = mem.as_mut().write(&[*self]);
        Ok(mem)
    }

    fn deserialize(mem: Bytes) -> Result<Self, DeError> {
        let size = Self::type_size()
            .expect("Integers have static size");
        if mem.len() < size {
            return Err(DeError::UnexpectedEOB);
        } else if mem.len() > size {
            return Err(DeError::BufferNotParsed);
        }
        Ok(*mem.get(0).expect("Length already checked"))
    }

    fn size(&self) -> usize {
        mem::size_of::<u8>()
    }

    fn type_size() -> Option<usize> {
        Some(mem::size_of::<u8>())
    }
}

impl Nl for u16 {
    fn serialize(&self, mut mem: BytesMut) -> Result<BytesMut, SerError> {
        Ok(put_int!(*self, mem, write_u16))
    }

    fn deserialize(mem: Bytes) -> Result<Self, DeError> {
        Ok(get_int!(mem, read_u16))
    }

    fn size(&self) -> usize {
        mem::size_of::<u16>()
    }

    fn type_size() -> Option<usize> {
        Some(mem::size_of::<u16>())
    }
}

impl Nl for u32 {
    fn serialize(&self, mut mem: BytesMut) -> Result<BytesMut, SerError> {
        Ok(put_int!(*self, mem, write_u32))
    }

    fn deserialize(mem: Bytes) -> Result<Self, DeError> {
        Ok(get_int!(mem, read_u32))
    }

    fn size(&self) -> usize {
        mem::size_of::<u32>()
    }

    fn type_size() -> Option<usize> {
        Some(mem::size_of::<u32>())
    }
}

impl Nl for i32 {
    fn serialize(&self, mut mem: BytesMut) -> Result<BytesMut, SerError> {
        Ok(put_int!(*self, mem, write_i32))
    }

    fn deserialize(mem: Bytes) -> Result<Self, DeError> {
        Ok(get_int!(mem, read_i32))
    }

    fn size(&self) -> usize {
        mem::size_of::<i32>()
    }

    fn type_size() -> Option<usize> {
        Some(mem::size_of::<i32>())
    }
}

impl Nl for u64 {
    fn serialize(&self, mut mem: BytesMut) -> Result<BytesMut, SerError> {
        Ok(put_int!(*self, mem, write_u64))
    }

    fn deserialize(mem: Bytes) -> Result<Self, DeError> {
        Ok(get_int!(mem, read_u64))
    }

    fn size(&self) -> usize {
        mem::size_of::<u64>()
    }

    fn type_size() -> Option<usize> {
        Some(mem::size_of::<u64>())
    }
}

impl<'a> Nl for &'a [u8] {
    fn serialize(&self, mut mem: BytesMut) -> Result<BytesMut, SerError> {
        if mem.len() > self.size() {
            return Err(SerError::BufferNotFilled(mem));
        } else if mem.len() < self.size() {
            return Err(SerError::UnexpectedEOB(mem));
        }
        mem.copy_from_slice(self);
        Ok(mem)
    }

    fn deserialize(_m: Bytes) -> Result<Self, DeError> {
        unimplemented!()
    }

    fn size(&self) -> usize {
        self.len()
    }

    fn type_size() -> Option<usize> {
        None
    }
}

impl<'a> NlSlice<'a> for &'a [u8] {
    fn deserialize_from_slice(mem: &'a [u8]) -> Result<Self, DeError> {
        Ok(mem)
    }
}

impl Nl for Vec<u8> {
    fn serialize(&self, mem: BytesMut) -> Result<BytesMut, SerError> {
        self.as_slice().serialize(mem)
    }

    fn deserialize(mem: Bytes) -> Result<Self, DeError> {
        Ok(mem.to_vec())
    }

    fn size(&self) -> usize {
        self.len()
    }

    fn type_size() -> Option<usize> {
        None
    }
}

impl<'a> Nl for &'a str {
    fn serialize(&self, mut mem: BytesMut) -> Result<BytesMut, SerError> {
        if mem.len() > self.size() {
            return Err(SerError::BufferNotFilled(mem));
        } else if mem.len() < self.size() {
            return Err(SerError::UnexpectedEOB(mem));
        }
        match mem.as_mut().write(self.as_bytes()) {
            Ok(write_size) => {
                assert_eq!(write_size + 1, self.size());
                mem.as_mut()[write_size] = 0;
                Ok(mem)
            },
            Err(e) => Err(SerError::IOError(e, mem)),
        }
    }

    fn deserialize(_: Bytes) -> Result<Self, DeError> {
        Err(DeError::new("Use deserialize_from_slice"))
    }

    fn size(&self) -> usize {
        self.len() + 1
    }

    fn type_size() -> Option<usize> {
        None
    }
}

impl<'a> NlSlice<'a> for &'a str {
    fn deserialize_from_slice(mem: &'a [u8]) -> Result<Self, DeError> {
        match mem.last() {
            Some(0) => (),
            _ => return Err(DeError::NullError),
        };
        str::from_utf8(&mem[..mem.len() - 1])
            .map_err(|e| DeError::new(e.to_string()))

    }
}

impl Nl for String {
    fn serialize(&self, mem: BytesMut) -> Result<BytesMut, SerError> {
        self.as_str().serialize(mem)
    }

    fn deserialize(mem: Bytes) -> Result<Self, DeError> {
        Ok(<&str>::deserialize_from_slice(mem.as_ref())?.to_string())
    }

    fn size(&self) -> usize {
        self.len() + 1
    }

    fn type_size() -> Option<usize> {
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use byteorder::NativeEndian;

    #[test]
    fn test_nl_u8() {
        let v: u8 = 5;
        let mut s = BytesMut::from(&[0u8] as &[u8]);
        s = v.serialize(s).unwrap();
        assert_eq!(s[0], v);

        let mem = Bytes::from(&[5u8] as &[u8]);
        let v = u8::deserialize(mem).unwrap();
        assert_eq!(v, 5)
    }

    #[test]
    fn test_nl_u16() {
        let v: u16 = 6000;
        let mut desired_buffer = [0u8; 2];
        NativeEndian::write_u16(&mut desired_buffer, 6000);

        let mut ser_buffer = BytesMut::from(&[0u8; 2] as &[u8]);
        ser_buffer = v.serialize(ser_buffer).unwrap();
        assert_eq!(ser_buffer.as_ref(), &desired_buffer);

        let mut s = BytesMut::from(&[0u8; 2] as &[u8]);
        NativeEndian::write_u16(s.as_mut(), 6000);
        u16::deserialize(s.freeze()).unwrap();
        assert_eq!(v, 6000);
    }

    #[test]
    fn test_nl_u32() {
        let v: u32 = 600_000;
        let mut s = [0u8; 4];
        NativeEndian::write_u32(&mut s, 600_000);

        let mut s_test = BytesMut::from(&[0u8; 4] as &[u8]);
        s_test = v.serialize(s_test).unwrap();
        assert_eq!(&s, s_test.as_ref());

        let mut s = BytesMut::from(&[0u8; 4] as &[u8]);
        NativeEndian::write_u32(s.as_mut(), 600_000);
        let v = u32::deserialize(s.freeze()).unwrap();
        assert_eq!(v, 600_000)
    }

    #[test]
    fn test_nl_u64() {
        let test_int: u64 = 12_345_678_901_234;
        let mut expected_serial = [0u8; 8];
        NativeEndian::write_u64(&mut expected_serial, test_int);

        let mut test_serial = BytesMut::from(&[0u8; 8] as &[u8]);
        test_serial = test_int.serialize(test_serial).unwrap();
        assert_eq!(&expected_serial, test_serial.as_ref());

        let mut buffer = [0u8; 8];
        NativeEndian::write_u64(&mut buffer, test_int);
        let deserialed_int = u64::deserialize(Bytes::from(&buffer as &[u8])).unwrap();
        assert_eq!(test_int, deserialed_int);
    }

    #[test]
    fn test_nl_slice() {
        let v: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9];
        let mut s = BytesMut::from(&[0u8; 9] as &[u8]);
        s = v.serialize(s).unwrap();
        assert_eq!(v, s.as_ref());

        let s: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 0, 0];
        let v = <&[u8]>::deserialize_from_slice(s).unwrap();
        assert_eq!(v, s);
    }

    #[test]
    fn test_nl_vec() {
        let v = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
        let mut s = BytesMut::from(&[0u8; 9] as &[u8]);
        s = v.serialize(s).unwrap();
        assert_eq!(v, s.to_vec());

        let s: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9];
        let v = Vec::<u8>::deserialize(Bytes::from(s)).unwrap();
        assert_eq!(v.as_slice(), s);
    }

    #[test]
    fn test_nl_str() {
        let s = "AAAAA";
        let mut sl = BytesMut::from(&[0u8; 6] as &[u8]);
        sl = s.serialize(sl).unwrap();
        assert_eq!(&[65, 65, 65, 65, 65, 0], sl.as_ref());

        let s = &[65u8, 65, 65, 65, 65, 65, 0] as &[u8];
        let string = <&str>::deserialize_from_slice(s).unwrap();
        assert_eq!(string, "AAAAAA")
    }

    #[test]
    fn test_nl_string() {
        let mut s = "AAAAA".to_string();
        let mut sl = BytesMut::from(&[0u8; 6] as &[u8]);
        sl = s.serialize(sl).unwrap();
        s.push('\0');
        assert_eq!(s.as_bytes(), sl.as_ref());

        let s = "AAAAAA\0";
        let string = String::deserialize(Bytes::from(s.as_bytes())).unwrap();
        assert_eq!(s[..s.len() - 1], string.to_string())
    }
}
