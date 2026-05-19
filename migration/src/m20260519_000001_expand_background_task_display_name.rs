//! 数据库迁移：放宽后台任务显示名长度，避免合法文件名加任务前缀后超出列限制。

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::DbBackend;

#[derive(DeriveMigrationName)]
pub struct Migration;

const OLD_DISPLAY_NAME_LEN: u32 = 255;
const NEW_DISPLAY_NAME_LEN: u32 = 512;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        alter_display_name_column(manager, NEW_DISPLAY_NAME_LEN).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        alter_display_name_column(manager, OLD_DISPLAY_NAME_LEN).await
    }
}

async fn alter_display_name_column(
    manager: &SchemaManager<'_>,
    display_name_len: u32,
) -> Result<(), DbErr> {
    match manager.get_database_backend() {
        DbBackend::Sqlite => Ok(()),
        DbBackend::Postgres | DbBackend::MySql => {
            manager
                .alter_table(
                    Table::alter()
                        .table(BackgroundTasks::Table)
                        .modify_column(
                            ColumnDef::new(BackgroundTasks::DisplayName)
                                .string_len(display_name_len)
                                .not_null()
                                .to_owned(),
                        )
                        .to_owned(),
                )
                .await
        }
        backend => Err(DbErr::Migration(format!(
            "unsupported database backend for background task display_name migration: {backend:?}"
        ))),
    }
}

#[derive(DeriveIden)]
enum BackgroundTasks {
    Table,
    DisplayName,
}
