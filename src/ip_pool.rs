//
// ip_pool.rs
// Copyright (C) 2022 matthew <matthew@WINDOWS-05HIC4F>
// Distributed under terms of the MIT license.
//

use std::{
    fmt::Display,
    net::{AddrParseError, Ipv4Addr},
    num::ParseIntError,
    process::Command,
    str::FromStr,
    string::FromUtf8Error,
};

use rand::Rng;
use rocket::serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct IPAddrPool {
    addr: Ipv4Addr,
    bits: u8,
}

pub struct IpHold<'a> {
    addr: Ipv4Addr,
    del: bool,
    dev: &'a str,
}

impl<'a> IpHold<'a> {
    pub fn ip<'b>(&'b self) -> &'b Ipv4Addr {
        &self.addr
    }

    pub fn drop(self) -> Result<(), PoolError> {
        if self.del {
            let tmp = Command::new("ip")
                .args(["addr", "del", "dev", self.dev, &format!("{}/32", self.addr)])
                .output()?;
            if !matches!(tmp.status.code(), Some(0) | None) {
                return Err(PoolError::FailedToSetIP);
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum PoolError {
    Io(std::io::Error),
    Utf8(FromUtf8Error),
    IP(AddrParseError),
    NoIpFound,
    FailedToSetIP,
}

impl From<std::io::Error> for PoolError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<FromUtf8Error> for PoolError {
    fn from(e: FromUtf8Error) -> Self {
        Self::Utf8(e)
    }
}
impl From<AddrParseError> for PoolError {
    fn from(e: AddrParseError) -> Self {
        Self::IP(e)
    }
}

impl IPAddrPool {
    fn get_rng_ip(&self) -> Ipv4Addr {
        let addr: u32 = self.addr.into();
        let num: u32 = rand::thread_rng().gen();
        let addr = addr ^ (num & !(!0 << self.bits));
        addr.into()
    }

    pub fn create_ip<'a>(&self, dev: &'a str) -> Result<IpHold<'a>, PoolError> {
        let ip = self.get_rng_ip();
        let tmp = Command::new("ip")
            .args(["addr", "add", "dev", dev, &format!("{}/32", ip)])
            .output()?;
        if !matches!(tmp.status.code(), Some(0) | None) {
            Err(PoolError::FailedToSetIP)
        } else {
            Ok(IpHold {
                addr: ip,
                del: true,
                dev,
            })
        }
    }

    pub fn default_ip<'a>(dev: &'a str) -> Result<IpHold<'a>, PoolError> {
        let tmp = Command::new("ip")
            .args(["addr", "show", "dev", dev])
            .output()?;
        let out = String::from_utf8(tmp.stdout)?;
        let mut words = out.split_whitespace();
        let _ = words.find(|&s| s == "inet");
        let (addr, _net) = words
            .next()
            .ok_or(PoolError::NoIpFound)?
            .split_once("/")
            .expect("Malformed ip addr output");
        let addr = addr.parse()?;
        if !matches!(tmp.status.code(), Some(0) | None) {
            Err(PoolError::FailedToSetIP)
        } else {
            Ok(IpHold {
                addr,
                del: false,
                dev,
            })
        }
    }
}

#[derive(Debug, Clone)]
pub enum IPAddrPoolError {
    IP(AddrParseError),
    Bits(ParseIntError),
    OutOfBounds(u8),
}

impl From<AddrParseError> for IPAddrPoolError {
    fn from(e: AddrParseError) -> Self {
        Self::IP(e)
    }
}

impl From<ParseIntError> for IPAddrPoolError {
    fn from(e: ParseIntError) -> Self {
        Self::Bits(e)
    }
}

impl Display for IPAddrPoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl TryFrom<String> for IPAddrPool {
    type Error = IPAddrPoolError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let (ip, bits) = value.split_once("/").unwrap_or((value.as_str(), "32"));
        let addr = Ipv4Addr::from_str(ip)?;
        let bits = bits.parse()?;
        if bits > 32 {
            Err(IPAddrPoolError::OutOfBounds(bits))
        } else {
            Ok(Self { addr, bits })
        }
    }
}

impl Into<String> for IPAddrPool {
    fn into(self) -> String {
        format!("{}/{}", self.addr, self.bits)
    }
}
