use sea_orm_migration::prelude::*;

use crate::iden::CrateIden;

/// Adds catalog-grouping columns to the `krate` table so related crates can be
/// grouped into a named collection and the entry-point crate(s) distinguished
/// from internal dependency crates. Populated from `[package.metadata.kellnr]`
/// in the published crate's `Cargo.toml` (see `kellnr-registry`).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // One column per ALTER statement — SQLite only supports a single
        // ADD COLUMN per ALTER TABLE.
        manager
            .alter_table(
                Table::alter()
                    .table(CrateIden::Table)
                    .add_column(ColumnDef::new(CrateIden::Collection).text().null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(CrateIden::Table)
                    .add_column(
                        ColumnDef::new(CrateIden::CollectionPrimary)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(CrateIden::Table)
                    .drop_column(CrateIden::CollectionPrimary)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(CrateIden::Table)
                    .drop_column(CrateIden::Collection)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
