//! Request signing for the NetEase Cloud Music Open Platform.
//!
//! The platform authenticates every call with an RSA signature over the common
//! parameters. Per the official docs (data-auth `getSignCheckContent` +
//! `rsa256Sign`), the algorithm is:
//!
//!   1. Collect the common params into a key-ascending sorted map.
//!   2. Join as `k1=v1&k2=v2&...` using RAW values (no URL encoding).
//!   The `sign` key itself must be excluded.
//!3. Sign with a PKCS#8 2048-bit RSA private key using SHA256WithRSA.
//!   4. Base64-encode the signature and send it as the `sign` param.
//!
//! Verified byte-identical against a real platform client's traffic.

use base64::Engine as _;
use rsa::pkcs1v15::SigningKey;
use rsa::pkcs8::DecodePrivateKey;
use rsa::sha2::Sha256;
use rsa::signature::{SignatureEncoding, Signer};
use rsa::RsaPrivateKey;
use std::collections::BTreeMap;

/// Fixed value required by the platform for the `signType` common param.
pub const SIGN_TYPE: &str = "RSA_SHA256";

/// Parse a PKCS#8 private key supplied as a single-line base64 string
/// (PEM header/footer and newlines stripped), which is how the console
/// and our `.env` store it. Also tolerates a full PEM block.
pub fn parse_private_key(raw: &str) -> Result<RsaPrivateKey, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("未配置 Private Key（私钥）".to_string());
    }

    if trimmed.contains("-----BEGIN") {
        return RsaPrivateKey::from_pkcs8_pem(trimmed)
            .map_err(|e| format!("解析 PKCS#8 PEM 私钥失败: {e}"));
  }

    // Single-line base64 → DER.
    let compact: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
    let der = base64::engine::general_purpose::STANDARD
        .decode(compact.as_bytes())
      .map_err(|e| format!("私钥 base64 解码失败: {e}"))?;

    RsaPrivateKey::from_pkcs8_der(&der).map_err(|e| format!("解析 PKCS#8 私钥失败: {e}"))
}

/// Build the exact string that gets signed. Public for testing.
///
/// `params` is a BTreeMap so iteration is already key-ascending, matching the
/// `TreeMap` + `Collections.sort` in the official Java sample.
pub fn sign_check_content(params: &BTreeMap<String, String>) -> String {
    params
        .iter()
        .filter(|(k, _)| k.as_str() != "sign")
        .map(|(k, v)| format!("{k}={v}"))
     .collect::<Vec<_>>()
        .join("&")
}

/// Sign the common params, returning the base64 signature for the `sign` param.
pub fn sign_params(params: &BTreeMap<String, String>, key: &RsaPrivateKey) -> String {
    let content = sign_check_content(params);
    let signing_key = SigningKey::<Sha256>::new(key.clone());
    let signature = signing_key.sign(content.as_bytes());
    base64::engine::general_purpose::STANDARD.encode(signature.to_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_content_is_sorted_and_excludes_sign() {
        let mut p = BTreeMap::new();
        p.insert("timestamp".to_string(), "1695697190151".to_string());
        p.insert("appId".to_string(), "abc".to_string());
        p.insert("signType".to_string(), SIGN_TYPE.to_string());
    p.insert("sign".to_string(), "should-be-dropped".to_string());

    assert_eq!(
            sign_check_content(&p),
   "appId=abc&signType=RSA_SHA256&timestamp=1695697190151"
  );
    }

    #[test]
    fn sign_content_uses_raw_unencoded_values() {
        let mut p = BTreeMap::new();
   p.insert("bizContent".to_string(), r#"{"limit":30}"#.to_string());
        assert_eq!(sign_check_content(&p), r#"bizContent={"limit":30}"#);
    }
}
