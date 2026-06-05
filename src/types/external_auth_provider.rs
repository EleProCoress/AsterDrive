use sea_orm::DeriveValueType;
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

/// Raw JSON object stored in `external_auth_providers.options`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, DeriveValueType)]
pub struct StoredExternalAuthProviderOptions(pub String);

impl StoredExternalAuthProviderOptions {
    pub const EMPTY_JSON: &str = "{}";

    pub fn empty() -> Self {
        Self(Self::EMPTY_JSON.to_string())
    }
}

impl AsRef<str> for StoredExternalAuthProviderOptions {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for StoredExternalAuthProviderOptions {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<StoredExternalAuthProviderOptions> for String {
    fn from(value: StoredExternalAuthProviderOptions) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct ExternalAuthProviderOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub microsoft: Option<MicrosoftExternalAuthProviderOptions>,
}

impl ExternalAuthProviderOptions {
    pub fn normalized(mut self) -> Self {
        if let Some(microsoft) = self.microsoft.take() {
            self.microsoft = microsoft.normalized();
        }
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct MicrosoftExternalAuthProviderOptions {
    pub tenant: String,
}

impl MicrosoftExternalAuthProviderOptions {
    pub fn new(tenant: impl Into<String>) -> Self {
        Self {
            tenant: tenant.into(),
        }
    }

    fn normalized(self) -> Option<Self> {
        let tenant = self.tenant.trim().to_string();
        (!tenant.is_empty()).then_some(Self { tenant })
    }
}

pub fn parse_external_auth_provider_options(options: &str) -> ExternalAuthProviderOptions {
    serde_json::from_str::<ExternalAuthProviderOptions>(options)
        .unwrap_or_else(|error| {
            if !options.is_empty() && options != StoredExternalAuthProviderOptions::EMPTY_JSON {
                tracing::warn!("invalid external auth provider options JSON '{options}': {error}");
            }
            ExternalAuthProviderOptions::default()
        })
        .normalized()
}

pub fn serialize_external_auth_provider_options(
    options: &ExternalAuthProviderOptions,
) -> std::result::Result<StoredExternalAuthProviderOptions, serde_json::Error> {
    serde_json::to_string(&options.clone().normalized()).map(StoredExternalAuthProviderOptions)
}

#[cfg(test)]
mod tests {
    use super::{
        ExternalAuthProviderOptions, MicrosoftExternalAuthProviderOptions,
        parse_external_auth_provider_options, serialize_external_auth_provider_options,
    };

    #[test]
    fn serialize_options_trims_empty_microsoft_tenant() {
        let stored = serialize_external_auth_provider_options(&ExternalAuthProviderOptions {
            microsoft: Some(MicrosoftExternalAuthProviderOptions::new("  ")),
        })
        .expect("options should serialize");

        assert_eq!(stored.as_ref(), "{}");
    }

    #[test]
    fn parse_options_recovers_invalid_json_as_empty() {
        let parsed = parse_external_auth_provider_options("{not-json");

        assert_eq!(parsed, ExternalAuthProviderOptions::default());
    }
}
