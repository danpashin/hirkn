use ipnet::IpNet;
use std::{net::IpAddr, str::FromStr};

pub(crate) enum IP {
    Plain(IpAddr),
    Prefixed(IpNet),
}

impl FromStr for IP {
    type Err = ();

    fn from_str(address: &str) -> Result<Self, Self::Err> {
        if let Ok(ip) = address.parse::<IpAddr>() {
            return Ok(Self::Plain(ip));
        }

        if let Ok(prefixed) = address.parse::<IpNet>() {
            return Ok(Self::Prefixed(prefixed));
        }

        Err(())
    }
}
