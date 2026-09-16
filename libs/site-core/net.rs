//! Shared IP-address prefix-truncation helpers.
//!
//! Two independent call sites mask an address down to a network prefix at
//! different widths: rate-limit keying truncates IPv6 to /64 (keys the
//! allocation, not the individual host address) and Phase-0 request
//! logging truncates IPv4 to /24 and IPv6 to /48 (data minimisation on a
//! value that is never a rate-limit key). One masking primitive, several
//! callers, so the truncation widths can't silently drift into separate
//! implementations.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Zero every bit below the leading `prefix_len` bits of `addr`.
///
/// `prefix_len` is clamped to the address family's bit width (32 for
/// IPv4, 128 for IPv6) — an oversized value truncates nothing rather than
/// panicking.
pub fn truncate_to_prefix(addr: IpAddr, prefix_len: u32) -> IpAddr {
    match addr {
        IpAddr::V4(v4) => IpAddr::V4(truncate_v4(v4, prefix_len.min(32))),
        IpAddr::V6(v6) => IpAddr::V6(truncate_v6(v6, prefix_len.min(128))),
    }
}

fn truncate_v4(addr: Ipv4Addr, prefix_len: u32) -> Ipv4Addr {
    // prefix_len is 0..=32 here; the `== 0` branch avoids a `<< 32` shift,
    // which is out of range for u32 and panics in debug builds.
    let mask: u32 = if prefix_len == 0 {
        0
    } else {
        u32::MAX << (32 - prefix_len)
    };
    Ipv4Addr::from(u32::from(addr) & mask)
}

fn truncate_v6(addr: Ipv6Addr, prefix_len: u32) -> Ipv6Addr {
    // Same `== 0` guard as `truncate_v4`, scaled to 128 bits.
    let mask: u128 = if prefix_len == 0 {
        0
    } else {
        u128::MAX << (128 - prefix_len)
    };
    Ipv6Addr::from(u128::from(addr) & mask)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipv4_truncates_to_24() {
        let addr: IpAddr = "203.0.113.201".parse().unwrap();
        assert_eq!(
            truncate_to_prefix(addr, 24),
            "203.0.113.0".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn ipv6_truncates_to_64() {
        let addr: IpAddr = "2001:db8:1234:5678:aaaa:bbbb:cccc:dddd".parse().unwrap();
        assert_eq!(
            truncate_to_prefix(addr, 64),
            "2001:db8:1234:5678::".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn ipv6_truncates_to_48() {
        let addr: IpAddr = "2001:db8:1234:5678:aaaa:bbbb:cccc:dddd".parse().unwrap();
        assert_eq!(
            truncate_to_prefix(addr, 48),
            "2001:db8:1234::".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn prefix_zero_zeroes_the_whole_address() {
        let v4: IpAddr = "203.0.113.201".parse().unwrap();
        assert_eq!(
            truncate_to_prefix(v4, 0),
            "0.0.0.0".parse::<IpAddr>().unwrap()
        );

        let v6: IpAddr = "2001:db8::1".parse().unwrap();
        assert_eq!(truncate_to_prefix(v6, 0), "::".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn prefix_at_full_width_is_a_no_op() {
        let v4: IpAddr = "203.0.113.201".parse().unwrap();
        assert_eq!(truncate_to_prefix(v4, 32), v4);

        let v6: IpAddr = "2001:db8::1".parse().unwrap();
        assert_eq!(truncate_to_prefix(v6, 128), v6);
    }

    #[test]
    fn oversized_prefix_is_clamped_not_a_panic() {
        let v4: IpAddr = "203.0.113.201".parse().unwrap();
        assert_eq!(truncate_to_prefix(v4, 999), v4);

        let v6: IpAddr = "2001:db8::1".parse().unwrap();
        assert_eq!(truncate_to_prefix(v6, 999), v6);
    }
}
