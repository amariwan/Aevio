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

entity!(freelancers, "staffing_freelancers", {
    name: String, email: Option<String>, status: String, location: Option<String>,
    timezone: String, seniority: Option<String>, bio: Option<String>, created_at: i64, updated_at: i64
});
entity!(skills, "staffing_skills", { name: String, category: Option<String> });
entity!(availability, "staffing_availability", {
    freelancer_id: String, start_date: String, end_date: String, available_minutes: i64, status: String
});
entity!(requests, "staffing_requests", {
    project_id: Option<String>, role: String, skills_json: String, start_date: String,
    end_date: String, requested_minutes: i64, status: String, created_at: i64, updated_at: i64
});
entity!(assignments, "staffing_assignments", {
    freelancer_id: String, project_id: Option<String>, request_id: Option<String>, role: String,
    start_date: String, end_date: String, planned_minutes: i64, buy_rate_cents: i64,
    sell_rate_cents: i64, status: String
});
entity!(contracts, "staffing_contracts", {
    freelancer_id: String, kind: String, start_date: String, end_date: Option<String>,
    notice_days: i64, status: String, document_ref: Option<String>
});
entity!(pipeline, "staffing_pipeline", {
    freelancer_id: String, stage: String, next_action_at: Option<i64>, notes: Option<String>
});

pub mod freelancer_skills {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "staffing_freelancer_skills")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub freelancer_id: String,
        #[sea_orm(primary_key, auto_increment = false)]
        pub skill_id: String,
        pub level: i64,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}
