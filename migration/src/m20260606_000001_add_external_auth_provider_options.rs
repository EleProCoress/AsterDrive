//! Add provider-specific options JSON for external auth providers.

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

const MICROSOFT_LOGIN_HOST: &str = "login.microsoftonline.com";
const MICROSOFT_DEFAULT_TENANT: &str = "common";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(ExternalAuthProviders::Table)
                    .add_column(
                        ColumnDef::new(ExternalAuthProviders::Options)
                            .text()
                            .not_null()
                            .default("{}"),
                    )
                    .to_owned(),
            )
            .await?;

        backfill_microsoft_provider_options(manager).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(ExternalAuthProviders::Table)
                    .drop_column(ExternalAuthProviders::Options)
                    .to_owned(),
            )
            .await
    }
}

async fn backfill_microsoft_provider_options(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let select = Query::select()
        .column(ExternalAuthProviders::Id)
        .column(ExternalAuthProviders::IssuerUrl)
        .from(ExternalAuthProviders::Table)
        .and_where(Expr::col(ExternalAuthProviders::ProviderKind).eq("microsoft"))
        .to_owned();
    let rows = manager
        .get_connection()
        .query_all(&select)
        .await
        .map_err(|error| {
            DbErr::Migration(format!(
                "failed to load Microsoft external auth providers for options backfill: {error}"
            ))
        })?;

    for row in rows {
        let id = row.try_get_by_index::<i64>(0).map_err(|error| {
            DbErr::Migration(format!(
                "failed to decode Microsoft external auth provider id during options backfill: {error}"
            ))
        })?;
        let issuer_url = row.try_get_by_index::<Option<String>>(1).map_err(|error| {
            DbErr::Migration(format!(
                "failed to decode Microsoft external auth provider #{id} issuer_url during options backfill: {error}"
            ))
        })?;
        let Some(tenant) = microsoft_tenant_from_issuer_url(issuer_url.as_deref()) else {
            continue;
        };
        let options = serde_json::json!({
            "microsoft": {
                "tenant": tenant,
            },
        })
        .to_string();

        manager
            .get_connection()
            .execute(
                &Query::update()
                    .table(ExternalAuthProviders::Table)
                    .value(ExternalAuthProviders::Options, options)
                    .value(ExternalAuthProviders::IssuerUrl, Option::<String>::None)
                    .and_where(Expr::col(ExternalAuthProviders::Id).eq(id))
                    .to_owned(),
            )
            .await
            .map_err(|error| {
                DbErr::Migration(format!(
                    "failed to backfill Microsoft external auth provider #{id} options: {error}"
                ))
            })?;
    }

    Ok(())
}

fn microsoft_tenant_from_issuer_url(value: Option<&str>) -> Option<String> {
    let value = value.map(str::trim).filter(|value| !value.is_empty())?;
    if value == MICROSOFT_DEFAULT_TENANT
        || value == "organizations"
        || value == "consumers"
        || is_microsoft_tenant_id(value)
    {
        return Some(value.to_string());
    }

    let path = value
        .strip_prefix("https://")
        .and_then(|value| value.strip_prefix(MICROSOFT_LOGIN_HOST))
        .and_then(|value| value.strip_prefix('/'))?;
    if path.contains('?') || path.contains('#') {
        return None;
    }
    let path = path.trim_end_matches('/');
    let segments = path.split('/').collect::<Vec<_>>();
    if segments.len() != 2 || segments[1] != "v2.0" {
        return None;
    }
    let tenant = segments[0];
    if tenant == MICROSOFT_DEFAULT_TENANT
        || tenant == "organizations"
        || tenant == "consumers"
        || is_microsoft_tenant_id(tenant)
    {
        Some(tenant.to_string())
    } else {
        None
    }
}

fn is_microsoft_tenant_id(value: &str) -> bool {
    const LEN: usize = 36;
    const HYPHEN_POSITIONS: [usize; 4] = [8, 13, 18, 23];

    value.len() == LEN
        && value.chars().enumerate().all(|(index, ch)| {
            if HYPHEN_POSITIONS.contains(&index) {
                ch == '-'
            } else {
                ch.is_ascii_hexdigit()
            }
        })
}

#[derive(DeriveIden)]
enum ExternalAuthProviders {
    Table,
    Id,
    ProviderKind,
    Options,
    IssuerUrl,
}
