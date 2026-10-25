use sea_orm::entity::prelude::*;
use sea_orm::sea_query::OnConflict;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, DatabaseConnection, DbErr, EntityTrait, QueryFilter,
    QueryOrder, TransactionTrait,
};

use crate::database::entities::staffing::{
    assignments, availability, contracts, freelancer_skills, freelancers, pipeline, requests,
    skills,
};

pub struct StaffingRepository<'db> {
    db: &'db DatabaseConnection,
}

pub type MatchData = (
    requests::Model,
    Vec<freelancers::Model>,
    Vec<skills::Model>,
    Vec<freelancer_skills::Model>,
    Vec<availability::Model>,
);

impl<'db> StaffingRepository<'db> {
    pub fn new(db: &'db DatabaseConnection) -> Self {
        Self { db }
    }
    pub fn freelancers(&self) -> Result<Vec<freelancers::Model>, DbErr> {
        freelancers::Entity::find()
            .order_by_asc(freelancers::Column::Name)
            .all(self.db)
    }
    pub fn freelancer(&self, id: &str) -> Result<Option<freelancers::Model>, DbErr> {
        freelancers::Entity::find_by_id(id.to_owned()).one(self.db)
    }
    pub fn skills(&self) -> Result<Vec<skills::Model>, DbErr> {
        skills::Entity::find()
            .order_by_asc(skills::Column::Name)
            .all(self.db)
    }
    pub fn availability(&self) -> Result<Vec<availability::Model>, DbErr> {
        availability::Entity::find()
            .order_by_asc(availability::Column::StartDate)
            .all(self.db)
    }
    pub fn requests(&self) -> Result<Vec<requests::Model>, DbErr> {
        requests::Entity::find()
            .order_by_asc(requests::Column::StartDate)
            .all(self.db)
    }
    pub fn assignments(&self) -> Result<Vec<assignments::Model>, DbErr> {
        assignments::Entity::find()
            .order_by_asc(assignments::Column::StartDate)
            .all(self.db)
    }
    pub fn contracts(&self) -> Result<Vec<contracts::Model>, DbErr> {
        contracts::Entity::find()
            .order_by_desc(contracts::Column::StartDate)
            .all(self.db)
    }
    pub fn pipeline(&self) -> Result<Vec<pipeline::Model>, DbErr> {
        pipeline::Entity::find()
            .order_by_asc(pipeline::Column::NextActionAt)
            .all(self.db)
    }

    pub fn upsert_freelancer(&self, row: freelancers::ActiveModel) -> Result<(), DbErr> {
        freelancers::Entity::insert(row)
            .on_conflict(
                OnConflict::column(freelancers::Column::Id)
                    .update_columns([
                        freelancers::Column::Name,
                        freelancers::Column::Email,
                        freelancers::Column::Status,
                        freelancers::Column::Location,
                        freelancers::Column::Timezone,
                        freelancers::Column::Seniority,
                        freelancers::Column::Bio,
                        freelancers::Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec(self.db)
            .map(|_| ())
    }
    pub fn upsert_skill(&self, row: skills::ActiveModel) -> Result<(), DbErr> {
        skills::Entity::insert(row)
            .on_conflict(
                OnConflict::column(skills::Column::Id)
                    .update_columns([skills::Column::Name, skills::Column::Category])
                    .to_owned(),
            )
            .exec(self.db)
            .map(|_| ())
    }
    pub fn upsert_availability(&self, row: availability::ActiveModel) -> Result<(), DbErr> {
        availability::Entity::insert(row)
            .on_conflict(
                OnConflict::column(availability::Column::Id)
                    .update_columns([
                        availability::Column::FreelancerId,
                        availability::Column::StartDate,
                        availability::Column::EndDate,
                        availability::Column::AvailableMinutes,
                        availability::Column::Status,
                    ])
                    .to_owned(),
            )
            .exec(self.db)
            .map(|_| ())
    }
    pub fn upsert_request(&self, row: requests::ActiveModel) -> Result<(), DbErr> {
        requests::Entity::insert(row)
            .on_conflict(
                OnConflict::column(requests::Column::Id)
                    .update_columns([
                        requests::Column::ProjectId,
                        requests::Column::Role,
                        requests::Column::SkillsJson,
                        requests::Column::StartDate,
                        requests::Column::EndDate,
                        requests::Column::RequestedMinutes,
                        requests::Column::Status,
                        requests::Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec(self.db)
            .map(|_| ())
    }
    pub fn upsert_assignment(&self, row: assignments::ActiveModel) -> Result<(), DbErr> {
        assignments::Entity::insert(row)
            .on_conflict(
                OnConflict::column(assignments::Column::Id)
                    .update_columns([
                        assignments::Column::FreelancerId,
                        assignments::Column::ProjectId,
                        assignments::Column::RequestId,
                        assignments::Column::Role,
                        assignments::Column::StartDate,
                        assignments::Column::EndDate,
                        assignments::Column::PlannedMinutes,
                        assignments::Column::BuyRateCents,
                        assignments::Column::SellRateCents,
                        assignments::Column::Status,
                    ])
                    .to_owned(),
            )
            .exec(self.db)
            .map(|_| ())
    }
    pub fn upsert_contract(&self, row: contracts::ActiveModel) -> Result<(), DbErr> {
        contracts::Entity::insert(row)
            .on_conflict(
                OnConflict::column(contracts::Column::Id)
                    .update_columns([
                        contracts::Column::FreelancerId,
                        contracts::Column::Kind,
                        contracts::Column::StartDate,
                        contracts::Column::EndDate,
                        contracts::Column::NoticeDays,
                        contracts::Column::Status,
                        contracts::Column::DocumentRef,
                    ])
                    .to_owned(),
            )
            .exec(self.db)
            .map(|_| ())
    }
    pub fn upsert_pipeline(&self, row: pipeline::ActiveModel) -> Result<(), DbErr> {
        pipeline::Entity::insert(row)
            .on_conflict(
                OnConflict::column(pipeline::Column::Id)
                    .update_columns([
                        pipeline::Column::FreelancerId,
                        pipeline::Column::Stage,
                        pipeline::Column::NextActionAt,
                        pipeline::Column::Notes,
                    ])
                    .to_owned(),
            )
            .exec(self.db)
            .map(|_| ())
    }

    pub fn replace_skills(&self, freelancer_id: &str, names: &[String]) -> Result<(), DbErr> {
        let tx = self.db.begin()?;
        freelancer_skills::Entity::delete_many()
            .filter(freelancer_skills::Column::FreelancerId.eq(freelancer_id.to_owned()))
            .exec(&tx)?;
        for name in names
            .iter()
            .map(|name| name.trim())
            .filter(|name| !name.is_empty())
        {
            let skill_id = match skills::Entity::find()
                .filter(skills::Column::Name.eq(name.to_owned()))
                .one(&tx)?
            {
                Some(row) => row.id,
                None => {
                    let id = uuid::Uuid::now_v7().to_string();
                    skills::ActiveModel {
                        id: Set(id.clone()),
                        name: Set(name.to_owned()),
                        category: Set(None),
                    }
                    .insert(&tx)?;
                    id
                }
            };
            freelancer_skills::ActiveModel {
                freelancer_id: Set(freelancer_id.to_owned()),
                skill_id: Set(skill_id),
                level: Set(1),
            }
            .insert(&tx)?;
        }
        tx.commit()
    }

    pub fn match_data(&self, request_id: &str) -> Result<MatchData, DbErr> {
        let request = requests::Entity::find_by_id(request_id.to_owned())
            .one(self.db)?
            .ok_or_else(|| DbErr::RecordNotFound(request_id.to_owned()))?;
        Ok((
            request,
            freelancers::Entity::find()
                .filter(freelancers::Column::Status.eq("active"))
                .order_by_asc(freelancers::Column::Name)
                .all(self.db)?,
            skills::Entity::find().all(self.db)?,
            freelancer_skills::Entity::find().all(self.db)?,
            availability::Entity::find()
                .filter(availability::Column::Status.eq("available"))
                .all(self.db)?,
        ))
    }
}
