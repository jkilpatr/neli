extern crate neli;
#[cfg(feature = "async")]
extern crate tokio;

use std::env;

use neli::{consts, err::NlError, genl::Genlmsghdr, socket, U32BitFlag, U32Bitmask};

#[cfg(feature = "async")]
use tokio::prelude::{Future, Stream};

#[cfg(feature = "async")]
fn debug_stream() -> Result<(), NlError> {
    let mut args = env::args();
    let _ = args.next();
    let first_arg = args.next();
    let second_arg = args.next();
    let (family_name, mc_group_name) = match (first_arg, second_arg) {
        (Some(fam_name), Some(mc_name)) => (fam_name, mc_name),
        (_, _) => {
            println!("USAGE: genl_stream FAMILY_NAME MULTICAST_GROUP_NAME");
            std::process::exit(1)
        }
    };
    let mut s = socket::NlSocket::connect(consts::NlFamily::Generic, None, U32Bitmask::empty())?;
    let id = s.resolve_nl_mcast_group(&family_name, &mc_group_name)?;
    let flag = match U32BitFlag::new(id) {
        Ok(f) => f,
        Err(_) => {
            return Err(NlError::new(format!(
                "{} is too large of a group number",
                id
            )))
        }
    };
    s.add_mcast_membership(U32Bitmask::from(flag))?;
    let ss = neli::socket::tokio::NlSocket::<u16, Genlmsghdr<u8, u16>>::new(s)?;
    tokio::run(
        ss.for_each(|next| {
            println!("{:?}", next);
            Ok(())
        })
        .map(|_| ())
        .map_err(|_| ()),
    );
    Ok(())
}

#[cfg(not(feature = "async"))]
fn debug_stream() -> Result<(), neli::err::NlError> {
    let mut args = env::args();
    let _ = args.next();
    let first_arg = args.next();
    let second_arg = args.next();
    let (family_name, mc_group_name) = match (first_arg, second_arg) {
        (Some(fam_name), Some(mc_name)) => (fam_name, mc_name),
        (_, _) => {
            println!("USAGE: genl_stream FAMILY_NAME MULTICAST_GROUP_NAME");
            std::process::exit(1)
        }
    };
    let mut s = socket::NlSocket::connect(consts::NlFamily::Generic, None, U32Bitmask::empty())?;
    let id = s.resolve_nl_mcast_group(&family_name, &mc_group_name)?;
    let flag = match U32BitFlag::new(id) {
        Ok(f) => f,
        Err(_) => {
            return Err(NlError::new(format!(
                "{} is too large of a group number",
                id
            )))
        }
    };
    s.add_mcast_membership(U32Bitmask::from(flag))?;
    for next in s.iter::<u16, Genlmsghdr<u8, u16>>() {
        println!("{:?}", next?);
    }
    Ok(())
}

pub fn main() {
    #[cfg(feature = "async")]
    match debug_stream() {
        Ok(_) => (),
        Err(e) => {
            println!("{}", e);
        }
    };
    #[cfg(not(feature = "async"))]
    match debug_stream() {
        Ok(_) => (),
        Err(e) => {
            println!("{}", e);
        }
    };
}
