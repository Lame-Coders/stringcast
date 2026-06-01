use crate::api::KeyMaterialStore;

const SERVICE_NAME: &str = "stringcast";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyringStoreError {
    Unavailable,
    NotFound,
}

#[derive(Debug, Clone, Default)]
pub struct KeyringKeyMaterialStore;

impl KeyringKeyMaterialStore {
    pub fn set_key(
        &self,
        provider: &str,
        key_id: &str,
        secret: &str,
    ) -> Result<(), KeyringStoreError> {
        keyring::Entry::new(SERVICE_NAME, &keyring_account_name(provider, key_id))
            .map_err(|_| KeyringStoreError::Unavailable)?
            .set_password(secret)
            .map_err(|_| KeyringStoreError::Unavailable)
    }

    pub fn delete_key(&self, provider: &str, key_id: &str) -> Result<(), KeyringStoreError> {
        keyring::Entry::new(SERVICE_NAME, &keyring_account_name(provider, key_id))
            .map_err(|_| KeyringStoreError::Unavailable)?
            .delete_password()
            .map_err(|error| match error {
                keyring::Error::NoEntry => KeyringStoreError::NotFound,
                _ => KeyringStoreError::Unavailable,
            })
    }
}

impl KeyMaterialStore for KeyringKeyMaterialStore {
    fn key_material(&self, key_id: &str) -> Option<String> {
        let providers = ["gemini", "openai", "anthropic", "custom"];
        providers.iter().find_map(|provider| {
            keyring::Entry::new(SERVICE_NAME, &keyring_account_name(provider, key_id))
                .ok()
                .and_then(|entry| entry.get_password().ok())
        })
    }
}

pub fn keyring_account_name(provider: &str, key_id: &str) -> String {
    format!("stringcast::provider::{provider}::key::{key_id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_name_matches_spec_shape() {
        assert_eq!(
            keyring_account_name("openai", "abc"),
            "stringcast::provider::openai::key::abc"
        );
    }
}
