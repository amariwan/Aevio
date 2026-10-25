pub mod records {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "invoices")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub number: Option<String>,
        pub client_id: String,
        pub client_name: String,
        pub client_email: Option<String>,
        pub client_address: Option<String>,
        pub currency: String,
        pub status: String,
        pub issue_date: Option<String>,
        pub due_date: Option<String>,
        pub terms_days: Option<i64>,
        pub subject: Option<String>,
        pub notes: Option<String>,
        pub payment_instructions: Option<String>,
        pub from_name: String,
        pub from_address: String,
        pub from_email: Option<String>,
        pub from_phone: Option<String>,
        pub total_cents: i64,
        pub pdf: Option<Vec<u8>>,
        pub issued_at: Option<i64>,
        pub paid_at: Option<i64>,
        pub created_at: i64,
        pub updated_at: i64,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod lines {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "invoice_lines")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub invoice_id: String,
        pub position: i64,
        pub kind: String,
        pub entry_id: Option<String>,
        pub project_name: String,
        pub description: String,
        pub started_at: Option<i64>,
        pub ended_at: Option<i64>,
        pub quantity_hundredths: i64,
        pub unit: String,
        pub rate_cents: i64,
        pub amount_cents: i64,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod entries {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "time_entries")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub project_id: Option<String>,
        pub description: String,
        pub started_at: i64,
        pub ended_at: i64,
        pub status: String,
        pub billable: i64,
        pub invoice_id: Option<String>,
        pub deleted_at: Option<i64>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod projects {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "projects")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub client_id: Option<String>,
        pub name: String,
        pub hourly_rate: Option<f64>,
        pub deleted_at: Option<i64>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod clients {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "clients")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub default_rate: Option<f64>,
        pub deleted_at: Option<i64>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}
