macro_rules! entity {
    ($module:ident, $table:literal, { $( $field:ident : $ty:ty ),* $(,)? }) => {
        pub mod $module {
            use sea_orm::entity::prelude::*;
            #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
            #[sea_orm(table_name = $table)]
            pub struct Model {
                #[sea_orm(primary_key, auto_increment = false)]
                pub id: String,
                $(pub $field: $ty,)*
            }
            #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
            pub enum Relation {}
            impl ActiveModelBehavior for ActiveModel {}
        }
    };
}

entity!(projects, "projects", { client_id: Option<String>, name: String, budget_kind: String, budget_value: Option<f64>, hourly_rate: Option<f64> });
entity!(clients, "clients", { name: String });
entity!(time_entries, "time_entries", { started_at: i64, ended_at: i64, project_id: Option<String>, billable: i64, status: String, invoice_id: Option<String>, deleted_at: Option<i64> });
pub mod rate_cards {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "finance_rate_cards")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub project_id: String,
        pub bill_rate_cents: i64,
        pub cost_rate_cents: i64,
        pub currency: String,
        pub effective_from: i64,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}
entity!(contracts, "finance_contracts", { client_id: String, project_id: Option<String>, kind: String, currency: String, start_date: String, end_date: Option<String>, recurring_cents: i64, included_hours_hundredths: i64, overage_rate_cents: i64, status: String });
entity!(capacity_plans, "finance_capacity_plans", { project_id: Option<String>, person_key: String, period_start: i64, period_end: i64, planned_minutes: i64, created_at: i64, updated_at: i64 });
entity!(dashboard_shares, "dashboard_shares", { dashboard_id: String, token: String, permission: String, expires_at: Option<i64>, created_at: i64 });
