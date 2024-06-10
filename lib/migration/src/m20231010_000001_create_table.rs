use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    let db = manager.get_connection();

    db.execute_unprepared(
      r#"
      create table "image"(
        id             uuid         not null primary key default gen_random_uuid(),
        created        timestamptz  not null             default now(),
        updated        timestamptz  not null             default now(),
        path           varchar(255) not null unique,
        width          int          not null,
        height         int          not null,
        file_size      bigint       not null,
        thumbnail      bool         not null             default false,
        taken          timestamptz,
        modified       timestamptz
      );
    "#,
    )
    .await?;

    Ok(())
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .get_connection()
      .execute_unprepared(
        r#"
        DROP TABLE "image";
      "#,
      )
      .await?;

    Ok(())
  }
}
