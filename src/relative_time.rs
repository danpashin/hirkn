use serde::{de::Visitor, Deserialize, Deserializer};
use std::{fmt::Formatter, num::ParseIntError, str::FromStr, time::Duration};

pub(crate) enum ParseError {
    Empty,
    InvalidTimeUnit(String),
    ParseInt(ParseIntError),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum RelativeTime {
    Seconds(u64),
    Minutes(u64),
    Hours(u64),
    Days(u64),
}

impl From<RelativeTime> for Duration {
    fn from(value: RelativeTime) -> Self {
        match value {
            RelativeTime::Seconds(value) => Duration::from_secs(value),
            RelativeTime::Minutes(value) => Duration::from_secs(value * 60),
            RelativeTime::Hours(value) => Duration::from_secs(value * 60 * 60),
            RelativeTime::Days(value) => Duration::from_secs(value * 24 * 60 * 60),
        }
    }
}

impl FromStr for RelativeTime {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let input_len = value.len();
        if input_len < 2 {
            return Err(ParseError::Empty);
        }

        let (time, unit) = value.split_at(input_len - 1);
        let time = time.parse().map_err(ParseError::ParseInt)?;

        match unit {
            "s" => Ok(Self::Seconds(time)),
            "m" => Ok(Self::Minutes(time)),
            "h" => Ok(Self::Hours(time)),
            "d" => Ok(Self::Days(time)),
            _ => Err(ParseError::InvalidTimeUnit(unit.to_string())),
        }
    }
}

struct RelativeTimeVisitor;

impl<'de> Visitor<'de> for RelativeTimeVisitor {
    type Value = RelativeTime;

    fn expecting(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a string, containing integer value and char time postfix")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        RelativeTime::from_str(v)
            .map_err(|_| serde::de::Error::invalid_value(serde::de::Unexpected::Str(v), &self))
    }
}

impl<'de> Deserialize<'de> for RelativeTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(RelativeTimeVisitor)
    }
}

#[cfg(test)]
mod test {
    use super::RelativeTime;

    #[test]
    fn parse_from_string() {
        assert_eq!("4613s".parse().ok(), Some(RelativeTime::Seconds(4613)));
        assert_eq!("1d".parse().ok(), Some(RelativeTime::Days(1)));

        assert_eq!("1f".parse::<RelativeTime>().ok(), None);
    }

    #[test]
    fn deserialize() {
        #[derive(serde::Deserialize)]
        struct TestStruct {
            time: RelativeTime,
        }

        let string = r###"{"time":"3d"}"###;
        let object: TestStruct = serde_json::from_str(string).expect("Cannot deserialize");
        assert_eq!(object.time, RelativeTime::Days(3));
    }
}
