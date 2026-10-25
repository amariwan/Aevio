macro_rules! entity {
    ($module:ident, $table:literal, { $( $field:ident : $ty:ty ),* $(,)? }, $pk:ident) => {
        pub mod $module {
            use sea_orm::entity::prelude::*;
            #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
            #[sea_orm(table_name = $table)]
            pub struct Model {
                #[sea_orm(primary_key, auto_increment = false)]
                pub $pk: String,
                $(pub $field: $ty,)*
            }
            #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
            pub enum Relation {}
            impl ActiveModelBehavior for ActiveModel {}
        }
    };
}

entity!(documents, "documents", { title: String, content: String, created_at: i64, updated_at: i64 }, id);
entity!(notes, "notes", { title: String, body: String, created_at: i64, updated_at: i64 }, id);
entity!(comments, "comments", { target_type: String, target_id: String, body: String, author: String, created_at: i64 }, id);
entity!(attachments, "attachments", { target_type: String, target_id: String, name: String, path: String, mime_type: Option<String>, size: Option<i64>, created_at: i64 }, id);
entity!(mentions, "mentions", { target_type: String, target_id: String, handle: String, created_at: i64 }, id);
entity!(cycles, "cycles", { name: String, goal: String, starts_at: i64, ends_at: i64, status: String, created_at: i64, updated_at: i64 }, id);
entity!(initiatives, "initiatives", { name: String, description: String, status: String, target_date: Option<i64>, created_at: i64, updated_at: i64 }, id);

pub mod initiative_projects {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "initiative_projects")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub initiative_id: String,
        #[sea_orm(primary_key, auto_increment = false)]
        pub project_id: String,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}
