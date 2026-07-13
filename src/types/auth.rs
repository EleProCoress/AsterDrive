use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

/// 联系方式验证渠道
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(16))")]
#[serde(rename_all = "snake_case")]
pub enum VerificationChannel {
    #[sea_orm(string_value = "email")]
    Email,
    #[sea_orm(string_value = "phone")]
    Phone,
}

/// 联系方式验证用途
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
#[serde(rename_all = "snake_case")]
pub enum VerificationPurpose {
    #[sea_orm(string_value = "register_activation")]
    RegisterActivation,
    #[sea_orm(string_value = "contact_change")]
    ContactChange,
    #[sea_orm(string_value = "password_reset")]
    PasswordReset,
}

/// 外部认证提供商类型。
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter, DeriveActiveEnum, Serialize, Deserialize,
)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
#[serde(rename_all = "snake_case")]
pub enum ExternalAuthProviderKind {
    #[sea_orm(string_value = "oidc")]
    Oidc,
    #[serde(rename = "generic_oauth2")]
    #[sea_orm(string_value = "generic_oauth2")]
    GenericOAuth2,
    #[serde(rename = "github")]
    #[sea_orm(string_value = "github")]
    GitHub,
    #[serde(rename = "google")]
    #[sea_orm(string_value = "google")]
    Google,
    #[serde(rename = "microsoft")]
    #[sea_orm(string_value = "microsoft")]
    Microsoft,
    #[serde(rename = "qq")]
    #[sea_orm(string_value = "qq")]
    Qq,
}

impl ExternalAuthProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Oidc => "oidc",
            Self::GenericOAuth2 => "generic_oauth2",
            Self::GitHub => "github",
            Self::Google => "google",
            Self::Microsoft => "microsoft",
            Self::Qq => "qq",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "oidc" => Some(Self::Oidc),
            "generic_oauth2" => Some(Self::GenericOAuth2),
            "github" => Some(Self::GitHub),
            "google" => Some(Self::Google),
            "microsoft" => Some(Self::Microsoft),
            "qq" => Some(Self::Qq),
            _ => None,
        }
    }
}

impl std::str::FromStr for ExternalAuthProviderKind {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value).ok_or(())
    }
}

impl AsRef<str> for ExternalAuthProviderKind {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<ExternalAuthProviderKind> for aster_forge_external_auth::ExternalAuthProviderKind {
    fn from(value: ExternalAuthProviderKind) -> Self {
        match value {
            ExternalAuthProviderKind::Oidc => Self::Oidc,
            ExternalAuthProviderKind::GenericOAuth2 => Self::GenericOAuth2,
            ExternalAuthProviderKind::GitHub => Self::GitHub,
            ExternalAuthProviderKind::Google => Self::Google,
            ExternalAuthProviderKind::Microsoft => Self::Microsoft,
            ExternalAuthProviderKind::Qq => Self::Qq,
        }
    }
}

impl From<aster_forge_external_auth::ExternalAuthProviderKind> for ExternalAuthProviderKind {
    fn from(value: aster_forge_external_auth::ExternalAuthProviderKind) -> Self {
        match value {
            aster_forge_external_auth::ExternalAuthProviderKind::Oidc => Self::Oidc,
            aster_forge_external_auth::ExternalAuthProviderKind::GenericOAuth2 => {
                Self::GenericOAuth2
            }
            aster_forge_external_auth::ExternalAuthProviderKind::GitHub => Self::GitHub,
            aster_forge_external_auth::ExternalAuthProviderKind::Google => Self::Google,
            aster_forge_external_auth::ExternalAuthProviderKind::Microsoft => Self::Microsoft,
            aster_forge_external_auth::ExternalAuthProviderKind::Qq => Self::Qq,
        }
    }
}

/// 外部认证协议族。
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
#[serde(rename_all = "snake_case")]
pub enum ExternalAuthProtocol {
    #[sea_orm(string_value = "oidc")]
    Oidc,
    #[serde(rename = "oauth2")]
    #[sea_orm(string_value = "oauth2")]
    OAuth2,
}

impl ExternalAuthProtocol {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Oidc => "oidc",
            Self::OAuth2 => "oauth2",
        }
    }
}

impl From<ExternalAuthProtocol> for aster_forge_external_auth::ExternalAuthProtocol {
    fn from(value: ExternalAuthProtocol) -> Self {
        match value {
            ExternalAuthProtocol::Oidc => Self::Oidc,
            ExternalAuthProtocol::OAuth2 => Self::OAuth2,
        }
    }
}

impl From<aster_forge_external_auth::ExternalAuthProtocol> for ExternalAuthProtocol {
    fn from(value: aster_forge_external_auth::ExternalAuthProtocol) -> Self {
        match value {
            aster_forge_external_auth::ExternalAuthProtocol::Oidc => Self::Oidc,
            aster_forge_external_auth::ExternalAuthProtocol::OAuth2 => Self::OAuth2,
        }
    }
}

impl ExternalAuthProviderKind {
    pub fn default_protocol(self) -> ExternalAuthProtocol {
        match self {
            Self::Oidc => ExternalAuthProtocol::Oidc,
            Self::GenericOAuth2 => ExternalAuthProtocol::OAuth2,
            Self::GitHub => ExternalAuthProtocol::OAuth2,
            Self::Google => ExternalAuthProtocol::Oidc,
            Self::Microsoft => ExternalAuthProtocol::Oidc,
            Self::Qq => ExternalAuthProtocol::OAuth2,
        }
    }
}

/// 持久化 MFA factor 类型。
///
/// 这个枚举只描述会长期绑定到用户账号、并保存进 `mfa_factors.method` 的认证因子。
/// 目前只有 TOTP，因为它需要保存加密后的共享密钥并支持启用/删除等管理操作。
///
/// 注意不要把登录挑战里的临时验证方式加到这里：
/// - 恢复码独立保存在 `mfa_recovery_codes`，不是 factor 行；
/// - 邮箱验证码独立保存在 `mfa_email_codes`，是某次登录 flow 的短期 challenge code；
/// - 如果只是“本次 MFA challenge 可以用什么验证”，应使用下面的 `MfaMethod`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    schema(as = MfaPersistentFactorType)
)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(16))")]
#[serde(rename_all = "snake_case")]
pub enum MfaPersistentFactorMethod {
    #[sea_orm(string_value = "totp")]
    Totp,
}

impl MfaPersistentFactorMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Totp => "totp",
        }
    }
}

/// MFA challenge 可用验证方法。
///
/// 这个枚举描述“某一次登录 flow 允许用户拿什么来完成第二步验证”。
/// 它可以包含持久化 factor 之外的短期方法，所以范围比 `MfaPersistentFactorMethod` 更宽。
/// 例如 `EmailCode` 只代表当前登录 flow 中发送到已验证邮箱的一次性验证码，
/// 不代表用户有一个持久化的 email MFA factor。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    schema(as = MfaChallengeMethodType)
)]
#[serde(rename_all = "snake_case")]
pub enum MfaMethod {
    Totp,
    RecoveryCode,
    EmailCode,
}

impl MfaMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Totp => "totp",
            Self::RecoveryCode => "recovery_code",
            Self::EmailCode => "email_code",
        }
    }
}

/// MFA flow 的第一因子来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
#[serde(rename_all = "snake_case")]
pub enum MfaFirstFactor {
    #[sea_orm(string_value = "password")]
    Password,
    #[sea_orm(string_value = "external_auth")]
    ExternalAuth,
}

impl MfaFirstFactor {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Password => "password",
            Self::ExternalAuth => "external_auth",
        }
    }
}

/// JWT Token 类型（不存 DB）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum TokenType {
    Access,
    Refresh,
}

impl TokenType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Access => "access",
            Self::Refresh => "refresh",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ExternalAuthProtocol, ExternalAuthProviderKind};

    #[test]
    fn external_auth_provider_kinds_round_trip_through_forge() {
        let kinds = [
            ExternalAuthProviderKind::Oidc,
            ExternalAuthProviderKind::GenericOAuth2,
            ExternalAuthProviderKind::GitHub,
            ExternalAuthProviderKind::Google,
            ExternalAuthProviderKind::Microsoft,
            ExternalAuthProviderKind::Qq,
        ];

        for kind in kinds {
            let forge_kind: aster_forge_external_auth::ExternalAuthProviderKind = kind.into();
            assert_eq!(forge_kind.as_str(), kind.as_str());
            assert_eq!(ExternalAuthProviderKind::from(forge_kind), kind);
        }
    }

    #[test]
    fn external_auth_protocols_round_trip_through_forge() {
        for protocol in [ExternalAuthProtocol::Oidc, ExternalAuthProtocol::OAuth2] {
            let forge_protocol: aster_forge_external_auth::ExternalAuthProtocol = protocol.into();
            assert_eq!(forge_protocol.as_str(), protocol.as_str());
            assert_eq!(ExternalAuthProtocol::from(forge_protocol), protocol);
        }
    }

    #[test]
    fn external_auth_default_protocols_match_forge_contract() {
        for forge_kind in aster_forge_external_auth::ExternalAuthProviderKind::ALL {
            let product_kind = ExternalAuthProviderKind::from(forge_kind);
            let product_protocol = product_kind.default_protocol();
            let forge_protocol: aster_forge_external_auth::ExternalAuthProtocol =
                product_protocol.into();

            assert_eq!(forge_kind.default_protocol(), forge_protocol);
        }
    }
}
