use std::{collections::HashMap, convert::TryInto, fmt, ops::Deref, str::FromStr};
use combinator::complete;
use nom::{
    named, tag,
    Err::{
        Incomplete as ParseIncomplete,
        Error as ParseError,
        Failure as ParseFailure
    }, IResult, Needed, branch::alt, bytes::complete::{tag, take, take_while1}, character::{complete::one_of, is_digit}, combinator::{self, opt}, error::{Error, ErrorKind}};

use num_bigint::{BigInt, BigUint, Sign};

#[derive(Clone)]
pub struct NodeId([u8; 20]);

impl Deref for NodeId {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        let NodeId(slice) = self;
        return slice;
    }
}

impl NodeId {
    pub fn distance(&self, node_id: &NodeId) -> BigUint {
        let bn1 = BigUint::from_bytes_be(&self);
        let bn2 = BigUint::from_bytes_be(node_id);
        return bn1 ^ bn2;
    }

    pub fn closest(&self, node_ids: &[NodeId]) -> NodeId {
        match node_ids.iter().enumerate()
            .map(|(n, node_id)| (n, self.distance(node_id)))
            .min_by(|(_, dist1), (_, dist2)| dist1.cmp(dist2)) {
                Some((n, _)) => node_ids[n].clone(),
                None => self.clone(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct BencodingParseError;
impl fmt::Display for BencodingParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "failed to parse bencoding")
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Bencoding {
    String(String),
    Integer(BigInt),
    List(Vec<Bencoding>),
    Dictionary(HashMap<String, Bencoding>),
}

impl Bencoding {
    pub fn from_slice(input: &[u8]) -> Result<Bencoding, BencodingParseError> {
        match Bencoding::parse(input) {
            Ok((leftovers, bencoding)) => match leftovers.is_empty() {
                true => Ok(bencoding),
                false => Err(BencodingParseError{}),
            },
            Err(_) => Err(BencodingParseError{}),
        }
    }

    fn parse_bigint(input: &[u8]) -> IResult<&[u8], BigInt> {
        // TODO: reject leading zeroes and -0
        let (input, opt_sign) = opt(tag("-"))(input)?;
        let (input, digits) = take_while1(is_digit)(input)?;
        let sign = opt_sign.unwrap_or_default();
        let n_slice = [&sign[..], &digits[..]].concat();
        return match BigInt::from_str(&String::from_utf8_lossy(&n_slice)) {
            Ok(v) => Ok((input, v)),
            Err(_) => return Err(ParseError(Error{input, code: ErrorKind::IsNot})),
        };
    }

    fn parse_integer(input: &[u8]) -> IResult<&[u8], Bencoding> {
        let (input, _) = tag("i")(input)?;
        let (input, n) = Bencoding::parse_bigint(input)?;
        let (input, _) = Bencoding::parse_end(input)?;
        return Ok((input, Bencoding::Integer(n)));
    }

    fn parse_string(input: &[u8]) -> IResult<&[u8], Bencoding> {
        let (input, n) = Bencoding::parse_bigint(input)?;
        if n.sign() == Sign::Minus {
            return Err(ParseError(Error{input, code: ErrorKind::IsNot}));
        }
        let n_u32: u32 = match n.try_into() {
            Ok(v) => v,
            Err(_) => return Err(ParseError(Error{input, code: ErrorKind::IsNot})),
        };
        let (input, _) = tag(":")(input)?;
        let (input, s) = take(n_u32)(input)?;
        return Ok((input, Bencoding::String(String::from_utf8_lossy(s).into_owned())));
    }

    named!(parse_end, tag!("e"));

    fn parse_list(input: &[u8]) -> IResult<&[u8], Bencoding> {
        let (mut c_input, _) = tag("l")(input)?;
        let mut elems = Vec::new();
        loop {
            match Bencoding::parse_end(c_input) {
                Ok((leftovers, _)) => {
                    c_input = leftovers;
                    break;
                },
                Err(e) => match e {
                    ParseError(_) => (),
                    other => return Err(other),
                }
            };
            let (leftovers, elem) = Bencoding::parse(c_input)?;
            c_input = leftovers;
            elems.push(elem);
        }
        return Ok((c_input, Bencoding::List(elems)));
    }

    fn parse_dictionary(input: &[u8]) -> IResult<&[u8], Bencoding> {
        let (mut c_input, _) = tag("d")(input)?;
        let mut dict = HashMap::new();
        loop {
            match Bencoding::parse_end(c_input) {
                Ok((leftovers, _)) => {
                    c_input = leftovers;
                    break;
                },
                Err(e) => match e {
                    ParseError(_) => (),
                    other => return Err(other),
                }
            };
            let (leftovers, wrapped_key) = Bencoding::parse_string(c_input)?;
            c_input = leftovers;
            let key = match wrapped_key {
                Bencoding::String(k) => k,
                _ => return Err(ParseError(Error{input, code: ErrorKind::IsNot})),
            };
            let (leftovers, value) = Bencoding::parse(c_input)?;
            c_input = leftovers;
            dict.insert(key, value);
        }
        // TODO: test for alphasort using OrderedMap
        return Ok((c_input, Bencoding::Dictionary(dict)));
    }

    fn parse(input: &[u8]) -> IResult<&[u8], Bencoding> {
        Ok(alt((
            complete(Bencoding::parse_integer),
            complete(Bencoding::parse_list),
            complete(Bencoding::parse_dictionary),
            complete(Bencoding::parse_string),
        ))(input)?)
    }
}


struct MetaInfo {
    
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bencoding_integer() {
        let make_bencoded_bigint = |s| Bencoding::Integer(BigInt::from_str(s).unwrap()) ;
        let ev = Vec::new();
        let mut success_cases = Vec::new();
        success_cases.push(("i28e", Ok((ev.as_ref(), make_bencoded_bigint("28")))));
        success_cases.push((
                "i31337e",
                Ok((ev.as_ref(), make_bencoded_bigint("31337"))),
        ));
        success_cases.push((
                "i123456789123456789e",
                Ok((ev.as_ref(), make_bencoded_bigint("123456789123456789"))),
        ));
        success_cases.push((
                "i-123456789123456789e",
                Ok((ev.as_ref(), make_bencoded_bigint("-123456789123456789"))),
        ));
        for case in success_cases.iter() {
            assert_eq!(case.1, Bencoding::parse(&case.0.as_bytes()));
        }
    }

    #[test]
    fn test_bencoding_string() {
        let ev = Vec::new();
        let mut success_cases = Vec::new();
        success_cases.push(("3:cat", Ok((ev.as_ref(), Bencoding::String("cat".to_string())))));
        success_cases.push(("4:dogg", Ok((ev.as_ref(), Bencoding::String("dogg".to_string())))));
        let v5 = vec![b'5'];
        success_cases.push(("4:12345", Ok((v5.as_ref(), Bencoding::String("1234".to_string())))));
        for case in success_cases.iter() {
            assert_eq!(case.1, Bencoding::parse(&case.0.as_bytes()));
        }
    }

    #[test]
    fn test_bencoding_list() {
        let ev = Vec::new();
        let mut success_cases = Vec::new();
        success_cases.push((
            "l4:spam4:eggse",
            Ok((ev.as_ref(), Bencoding::List(vec![
                        Bencoding::String("spam".to_string()),
                        Bencoding::String("eggs".to_string())
            ]))),
        ));
        for case in success_cases.iter() {
            assert_eq!(case.1, Bencoding::parse(&case.0.as_bytes()));
        }
    }

    #[test]
    fn test_bencoding_dictionary() {
        let ev = Vec::new();
        let mut success_cases = Vec::new();
        let mut sc1_map = HashMap::new();
        sc1_map.insert("cow".to_string(), Bencoding::String("moo".to_string()));
        sc1_map.insert("spam".to_string(), Bencoding::String("eggs".to_string()));
        let sc1 = Bencoding::Dictionary(sc1_map);
        success_cases.push((
            "d3:cow3:moo4:spam4:eggse",
            Ok((ev.as_ref(), sc1)),
        ));
        for case in success_cases.iter() {
            assert_eq!(case.1, Bencoding::parse(&case.0.as_bytes()));
        }
    }
}
