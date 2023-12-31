use ipnet::{IpNet, Ipv4Net, Ipv6Net};
use serde::{de, Deserialize, Deserializer};
use std::{
    fmt::Formatter,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    str::FromStr,
};

#[derive(Debug, Eq, PartialEq, Hash)]
pub(crate) enum IP {
    Single(IpAddr),
    Network(IpNet),
}

impl IP {
    pub(crate) fn as_network(&self) -> Option<&IpNet> {
        match self {
            Self::Single(_) => None,
            Self::Network(net) => Some(net),
        }
    }
}

impl FromStr for IP {
    type Err = ();

    fn from_str(address: &str) -> Result<Self, Self::Err> {
        if let Ok(ip) = address.parse::<IpAddr>() {
            return Ok(Self::Single(ip));
        }

        if let Ok(prefixed) = address.parse::<IpNet>() {
            return Ok(Self::Network(prefixed));
        }

        Err(())
    }
}

impl From<IP> for nftables::expr::Expression {
    fn from(ip: IP) -> Self {
        use nftables::expr::{Expression, NamedExpression, Prefix};

        match ip {
            IP::Single(ip) => Self::String(ip.to_string()),
            IP::Network(ip_net) => {
                let prefix = Prefix {
                    addr: Box::new(Self::String(ip_net.network().to_string())),
                    len: u32::from(ip_net.prefix_len()),
                };

                Expression::Named(NamedExpression::Prefix(prefix))
            }
        }
    }
}

impl PartialEq<IpAddr> for IP {
    fn eq(&self, other: &IpAddr) -> bool {
        match self {
            Self::Single(single) => single == other,
            Self::Network(_) => false,
        }
    }
}

impl PartialEq<Ipv4Addr> for IP {
    fn eq(&self, other: &Ipv4Addr) -> bool {
        self.eq(&IpAddr::V4(*other))
    }
}

impl PartialEq<Ipv6Addr> for IP {
    fn eq(&self, other: &Ipv6Addr) -> bool {
        self.eq(&IpAddr::V6(*other))
    }
}

impl PartialEq<IpNet> for IP {
    fn eq(&self, other: &IpNet) -> bool {
        match self {
            Self::Network(net) => {
                net.addr() == other.addr() && net.prefix_len() == other.prefix_len()
            }
            Self::Single(_) => false,
        }
    }
}

impl PartialEq<Ipv4Net> for IP {
    fn eq(&self, other: &Ipv4Net) -> bool {
        self.eq(&IpNet::V4(*other))
    }
}

impl PartialEq<Ipv6Net> for IP {
    fn eq(&self, other: &Ipv6Net) -> bool {
        self.eq(&IpNet::V6(*other))
    }
}

impl<'de> Deserialize<'de> for IP {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct IPVisitor;

        impl<'vde> de::Visitor<'vde> for IPVisitor {
            type Value = IP;
            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("valid IPv4/IPv6 or valid CIDR notation")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                IP::from_str(v)
                    .map_err(|()| de::Error::unknown_variant(v, &["IPv4/IPv6", "CIDR-notation"]))
            }
        }

        deserializer.deserialize_string(IPVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::IP;
    use ipnet::{Ipv4Net, Ipv6Net};
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn parse_ipv4_single() {
        let parsed: IP = "127.0.0.1".parse().unwrap();
        let expected = Ipv4Addr::new(127, 0, 0, 1);

        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_ipv6_single() {
        let parsed: IP = "::1".parse().unwrap();
        let expected = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1);

        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_ipv4_network() {
        let parsed: IP = "127.0.0.1/32".parse().unwrap();

        let expected_ip = Ipv4Addr::new(127, 0, 0, 1);
        let expected_net = Ipv4Net::new(expected_ip, 32).unwrap();

        assert_eq!(parsed, expected_net);

        let parsed: IP = "127.0.0.1/24".parse().unwrap();
        assert_ne!(parsed, expected_net);
    }

    #[test]
    fn parse_ipv6_network() {
        let parsed: IP = "::1/128".parse().unwrap();

        let expected_ip = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1);
        let expected_net = Ipv6Net::new(expected_ip, 128).unwrap();

        assert_eq!(parsed, expected_net);

        let parsed: IP = "::1/64".parse().unwrap();
        assert_ne!(parsed, expected_net);
    }
}
