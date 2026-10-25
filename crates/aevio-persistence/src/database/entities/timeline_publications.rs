use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "timeline_publications")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub segment_id: i64,
    pub visibility: String,
    pub workspace_id: Option<String>,
    pub title_override: Option<String>,
    pub published_at: Option<i64>,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
