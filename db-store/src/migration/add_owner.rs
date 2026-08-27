//! Adds a nullable `owner` column to `records`: additive, so an
//! already-migrated database only gains the column.

use sea_orm_migration::prelude::*;

use super::Records;

pub(super) struct AddRecordOwner;

impl MigrationName for AddRecordOwner {
    fn name(&self) -> &str {
        "m20260827_000008_add_record_owner"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for AddRecordOwner {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Records::Table)
                    .add_column(ColumnDef::new(Records::Owner).text())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Records::Table)
                    .drop_column(Records::Owner)
                    .to_owned(),
            )
            .await
    }
}
