//! Track whether a failed background task can be manually retried.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(BackgroundTasks::Table)
                    .add_column(
                        ColumnDef::new(BackgroundTasks::FailureCanRetry)
                            .boolean()
                            .null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(BackgroundTasks::Table)
                    .drop_column(BackgroundTasks::FailureCanRetry)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum BackgroundTasks {
    Table,
    FailureCanRetry,
}
