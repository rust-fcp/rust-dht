// Copyright 2014 Dmitry "Divius" Tantsur <divius.inside@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
//

//! KRPC protocol bits as described in
//! [BEP 0005](http://www.bittorrent.org/beps/bep_0005.html).

use std::collections;

use bencode::{mod, FromBencode, ToBencode};
use bencode::util::ByteString;
use num;

use super::super::base;
use super::super::utils;


// TODO(divius): actually validate it
static ID_BYTE_SIZE: uint = 20;

/// Mapping String -> Bytes used in payload.
pub type PayloadDict = collections::TreeMap<String, Vec<u8>>;

/// Package payload in KRPC: either Query (request) or Response or Error.
pub enum Payload {
    /// Request to a node.
    Query(PayloadDict),
    /// Response to request.
    Response(PayloadDict),
    /// Error: code and string message.
    Error(i64, String)
}

/// KRPC package.
pub struct Package {
    /// Transaction ID generated by requester and passed back by responder.
    pub transaction_id: Vec<u8>,
    /// Package payload.
    pub payload: Payload,
    /// Sender Node (note that as per BEP 0005 it is stored in payload).
    pub sender: base::Node
}


fn id_to_netbytes(id: &num::BigUint) -> Vec<u8> {
    assert!(id.bits() <= ID_BYTE_SIZE * 8);

    let mut id_c = id.clone();
    let mask = FromPrimitive::from_u8(0xFF).unwrap();
    let mut result = Vec::from_elem(ID_BYTE_SIZE, 0);

    for i in result.iter_mut().rev() {
        let part = id_c & mask;
        *i = part.to_u8().unwrap();
        id_c = id_c >> 8;
    }

    result
}

fn id_from_netbytes(bytes: &[u8]) -> num::BigUint {
    let mut result: num::BigUint = FromPrimitive::from_int(0).unwrap();
    let mut shift = 0;
    for i in bytes.iter().rev() {
        let val: num::BigUint = FromPrimitive::from_u8(*i).unwrap();
        result = result + (val << shift);
        shift += 8;
    }
    result
}

impl ToBencode for base::Node {
    fn to_bencode(&self) -> bencode::Bencode {
        let mut result = id_to_netbytes(&self.id);
        result.push_all(utils::netaddr_to_netbytes(&self.address).as_slice());
        bencode::ByteString(result)
    }
}

impl FromBencode for base::Node {
    fn from_bencode(b: &bencode::Bencode) -> Option<base::Node> {
        match *b {
            bencode::ByteString(ref v) if v.len() == 26 => Some(base::Node {
                id: id_from_netbytes(v.slice(0, 20)),
                address: utils::netaddr_from_netbytes(v.slice(20, 26))
            }),
            _ => None
        }
    }
}

impl Package {
    fn payload_dict_to_bencode(&self, d: &PayloadDict) -> bencode::Bencode {
        let mut result: bencode::DictMap = d.iter().map(|(k, v)| {
            (ByteString::from_str(k.as_slice()), v.to_bencode())
        }).collect();
        result.insert(ByteString::from_str("id"), self.sender.to_bencode());
        bencode::Dict(result)
    }
}

impl ToBencode for Package {
    fn to_bencode(&self) -> bencode::Bencode {
        let mut result: bencode::DictMap = collections::TreeMap::new();

        result.insert(ByteString::from_str("tt"),
                      bencode::ByteString(self.transaction_id.clone()));
        let (typ, payload) = match self.payload {
            Query(ref d) => ("q", self.payload_dict_to_bencode(d)),
            Response(ref d) => ("r", self.payload_dict_to_bencode(d)),
            Error(code, ref s) => {
                let l = vec![code.to_bencode(), s.to_bencode()];
                ("e", bencode::List(l))
            }
        };
        result.insert(ByteString::from_str("y"), typ.to_string().to_bencode());
        result.insert(ByteString::from_str(typ), payload);

        bencode::Dict(result)
    }
}


#[cfg(test)]
mod test {
    use std::collections;

    use bencode::{mod, FromBencode, ToBencode};

    use super::super::super::base;
    use super::super::super::utils::test;

    use super::PayloadDict;
    use super::Error;
    use super::Package;
    use super::Payload;
    use super::Query;
    use super::Response;


    fn new_package(payload: Payload) -> Package {
        Package {
            transaction_id: vec![1, 2, 254, 255],
            sender: test::new_node(42),
            payload: payload
        }
    }

    fn common<'a>(b: &'a bencode::Bencode, typ: &str) -> &'a bencode::DictMap {
        match *b {
            bencode::Dict(ref d) => {
                let tt_val = &d[bencode::util::ByteString::from_str("tt")];
                match *tt_val {
                    bencode::ByteString(ref v) => {
                        assert_eq!(vec![1, 2, 254, 255], *v);
                    },
                    _ => fail!("unexpected {}", tt_val)
                };

                let y_val = &d[bencode::util::ByteString::from_str("y")];
                match *y_val {
                    bencode::ByteString(ref v) => {
                        assert_eq!(typ.as_bytes(), v.as_slice());
                    },
                    _ => fail!("unexpected {}", y_val)
                };

                d
            },
            _ => fail!("unexpected {}", b)
        }
    }

    fn dict<'a>(b: &'a bencode::Bencode, typ: &str) -> &'a bencode::DictMap {
        let d = common(b, typ);

        let typ_val = &d[bencode::util::ByteString::from_str(typ)];
        match *typ_val {
            bencode::Dict(ref m) => m,
            _ => fail!("unexpected {}", typ_val)
        }
    }

    fn list<'a>(b: &'a bencode::Bencode, typ: &str) -> &'a bencode::ListVec {
        let d = common(b, typ);

        let typ_val = &d[bencode::util::ByteString::from_str(typ)];
        match *typ_val {
            bencode::List(ref l) => l,
            _ => fail!("unexpected {}", typ_val)
        }
    }

    #[test]
    fn test_error_to_bencode() {
        let p = new_package(Error(10, "error".to_string()));
        let enc = p.to_bencode();
        let l = list(&enc, "e");
        assert_eq!(vec![bencode::Number(10),
                        "error".to_string().to_bencode()],
                   *l);
    }

    #[test]
    fn test_query_to_bencode() {
        let payload: PayloadDict = collections::TreeMap::new();
        let p = new_package(Query(payload));
        let enc = p.to_bencode();
        dict(&enc, "q");
        // TODO(divius): Moar tests
    }

    #[test]
    fn test_response_to_bencode() {
        let payload: PayloadDict = collections::TreeMap::new();
        let p = new_package(Response(payload));
        let enc = p.to_bencode();
        dict(&enc, "r");
        // TODO(divius): Moar tests
    }

    #[test]
    fn test_id_to_netbytes() {
        let id = test::uint_to_id(0x0A0B0C0D);
        let b = super::id_to_netbytes(&id);
        let mut expected = Vec::from_elem(16, 0u8);
        expected.push_all([0x0A, 0x0b, 0x0C, 0x0D]);
        assert_eq!(expected, b);
    }

    #[test]
    fn test_id_from_netbytes() {
        let mut bytes = Vec::from_elem(16, 0u8);
        bytes.push_all([0x0A, 0x0b, 0x0C, 0x0D]);
        let expected = test::uint_to_id(0x0A0B0C0D);
        let id = super::id_from_netbytes(bytes.as_slice());
        assert_eq!(expected, id);
    }

    #[test]
    fn test_node_to_bencode() {
        let n = test::new_node(42);
        let enc = n.to_bencode();
        let mut expected = Vec::from_elem(19, 0u8);
        expected.push_all([42, 127, 0, 0, 1, 31, 72]);
        assert_eq!(bencode::ByteString(expected), enc);
    }

    #[test]
    fn test_node_from_bencode() {
        let mut b = Vec::from_elem(19, 0u8);
        b.push_all([42, 127, 0, 0, 1, 0, 80]);
        let n: base::Node =
            FromBencode::from_bencode(&bencode::ByteString(b)).unwrap();
        assert_eq!(n.id, test::uint_to_id(42));
        assert_eq!(n.address.to_string().as_slice(), "127.0.0.1:80");
    }

    #[test]
    fn test_node_from_bencode_none() {
        let n: Option<base::Node> =
            FromBencode::from_bencode(&bencode::Number(42));
        assert!(n.is_none());
    }

    #[test]
    fn test_node_to_from_bencode() {
        let n = test::new_node(42);
        let enc = n.to_bencode();
        let n2: base::Node = FromBencode::from_bencode(&enc).unwrap();
        assert_eq!(n.id, n2.id);
        assert_eq!(n.address, n2.address);
    }
}
