//! Various utilities

use std::net;


/// Convert socket address to bytes in network order.
pub fn netaddr_to_netbytes(addr: &net::SocketAddr) -> Vec<u8> {
    match *addr {
        net::SocketAddr::V4(ref addr) => {
            let mut res = addr.ip().octets().to_vec();
            res.push((addr.port() >> 8) as u8);
            res.push((addr.port() & 0xFF) as u8);
            res
        },
        // TODO(divius): implement
        net::SocketAddr::V6(..) => panic!("IPv6 not implemented")
    }
}

/// Get socket address from netbytes.
pub fn netaddr_from_netbytes(bytes: &[u8]) -> net::SocketAddr {
    assert_eq!(6, bytes.len());
    net::SocketAddr::V4(net::SocketAddrV4::new(
        net::Ipv4Addr::new(bytes[0], bytes[1], bytes[2], bytes[3]),
       ((bytes[4] as u16) << 8) + bytes[5] as u16
    ))
}


#[cfg(test)]
pub mod test {
    use std::net;

    use num;
    use num::{FromPrimitive, ToPrimitive};

    use super::super::Node;


    pub static ADDR: &'static str = "127.0.0.1:8008";

    pub fn new_node(id: usize) -> Node {
        new_node_with_port(id, 8008)
    }

    pub fn new_node_with_port(id: usize, port: u16) -> Node {
        Node {
            id: FromPrimitive::from_usize(id).unwrap(),
            address: net::SocketAddr::V4(net::SocketAddrV4::new(
                net::Ipv4Addr::new(127, 0, 0, 1),
                port
            ))
        }
    }

    pub fn usize_to_id(id: usize) -> num::BigUint {
        FromPrimitive::from_usize(id).unwrap()
    }
}
