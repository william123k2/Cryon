use crate::errors::Result;
use md5::{Digest, Md5};
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Profile {
    pub name: String,
    pub uuid: String,
    pub kind: String,
}
impl Profile {
    pub fn offline(name: &str) -> Result<Self> {
        if !(3..=16).contains(&name.len())
            || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        {
            return Err("Nome offline: use 3–16 letras ASCII, números ou sublinhado.".into());
        }
        let hash = Md5::digest(format!("OfflinePlayer:{name}"));
        let mut bytes: [u8; 16] = hash.into();
        bytes[6] = (bytes[6] & 0x0f) | 0x30;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        Ok(Self {
            name: name.into(),
            uuid: uuid::Uuid::from_bytes(bytes).simple().to_string(),
            kind: "offline".into(),
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn java_uuid_compatibility() {
        assert_eq!(
            Profile::offline("Notch").expect("profile").uuid,
            "b50ad385829d3141a2167e7d7539ba7f"
        );
        assert!(Profile::offline("../abc").is_err());
    }
}
