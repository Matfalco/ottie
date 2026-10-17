//! Certificates trusted by hand (docs/decisions/0054): an FTPS server whose
//! certificate the system doesn't trust (a NAS or a router that made its
//! own) can be trusted for that exact certificate, by its SHA-256. A server
//! that shows another one is refused again.

use std::fmt;

use crate::error::Error;

/// Identifies a trusted certificate. A plain number, like [`crate::ViewId`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CertificateId(pub u32);

impl fmt::Display for CertificateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A server's certificate the person trusts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedCertificate {
    pub id: CertificateId,
    /// The server's name as in links, lowercase (an IPv6 address in brackets).
    pub host: String,
    /// SHA-256 of the certificate, 64 lowercase hex digits.
    pub fingerprint: String,
}

/// `host` and `fingerprint` as kept: lowercase, the fingerprint without
/// the colons or spaces it's often written with. [`Error::InvalidCertificate`]
/// if either isn't one.
pub fn normalized(host: &str, fingerprint: &str) -> Result<(String, String), Error> {
    let host = host.trim().to_ascii_lowercase();
    let valid_host = !host.is_empty()
        && host.len() <= 255
        && !host
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || "/@?#\\".contains(c));
    let fingerprint: String = fingerprint
        .chars()
        .filter(|c| !matches!(c, ':' | ' '))
        .map(|c| c.to_ascii_lowercase())
        .collect();
    let valid_fingerprint =
        fingerprint.len() == 64 && fingerprint.chars().all(|c| c.is_ascii_hexdigit());
    if valid_host && valid_fingerprint {
        Ok((host, fingerprint))
    } else {
        Err(Error::InvalidCertificate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_them_in_one_form() {
        let colons = vec!["AB"; 32].join(":");
        assert_eq!(
            normalized(" NAS.local ", &colons).unwrap(),
            ("nas.local".to_owned(), "ab".repeat(32))
        );
        assert!(normalized("nas", "ab").is_err());
        assert!(normalized("nas", &"zz".repeat(32)).is_err());
        assert!(normalized("", &"ab".repeat(32)).is_err());
        assert!(normalized("a b", &"ab".repeat(32)).is_err());
        assert!(normalized("[::1]", &"ab".repeat(32)).is_ok());
    }
}
