use sea_orm::entity::prelude::*;
use sea_orm::{ActiveValue::Set, DatabaseConnection, DbErr};

use crate::database::entities::invoice_profile::{self, invoices, sequences};

#[derive(Debug, Clone, PartialEq)]
pub struct ProfileRow {
    pub name: String,
    pub address: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub payment_instructions: Option<String>,
    pub default_notes: Option<String>,
    pub default_terms_days: i64,
    pub has_logo: bool,
    pub next_number: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProfileUpdate {
    pub name: String,
    pub address: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub payment_instructions: Option<String>,
    pub default_notes: Option<String>,
    pub default_terms_days: i64,
    pub next_number: Option<i64>,
}

pub struct InvoiceProfileRepository<'db> {
    db: &'db DatabaseConnection,
}

impl<'db> InvoiceProfileRepository<'db> {
    pub fn new(db: &'db DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn next_number(&self, year: i64) -> Result<i64, DbErr> {
        Ok(sequences::Entity::find_by_id(year)
            .one(self.db)?
            .map(|row| row.next_number)
            .unwrap_or(1))
    }

    pub fn get(&self, year: i64) -> Result<ProfileRow, DbErr> {
        let row = invoice_profile::Entity::find_by_id(1)
            .one(self.db)?
            .ok_or_else(|| DbErr::RecordNotFound("invoice profile missing".into()))?;
        Ok(ProfileRow {
            name: row.name,
            address: row.address,
            email: row.email,
            phone: row.phone,
            payment_instructions: row.payment_instructions,
            default_notes: row.default_notes,
            default_terms_days: row.default_terms_days,
            has_logo: row.logo.is_some(),
            next_number: self.next_number(year)?,
        })
    }

    pub fn update(&self, input: ProfileUpdate, year: i64, now: i64) -> Result<(), DbErr> {
        self.db
            .transaction::<_, _, DbErr>(|transaction| {
                let row = invoice_profile::Entity::find_by_id(1)
                    .one(transaction)?
                    .ok_or_else(|| DbErr::RecordNotFound("invoice profile missing".into()))?;
                let mut model: invoice_profile::ActiveModel = row.into();
                model.name = Set(input.name.clone());
                model.address = Set(input.address.clone());
                model.email = Set(input.email.clone());
                model.phone = Set(input.phone.clone());
                model.payment_instructions = Set(input.payment_instructions.clone());
                model.default_notes = Set(input.default_notes.clone());
                model.default_terms_days = Set(input.default_terms_days);
                model.updated_at = Set(now);
                model.update(transaction)?;
                if let Some(next_number) = input.next_number {
                    let existing = sequences::Entity::find_by_id(year).one(transaction)?;
                    if let Some(existing) = existing {
                        let mut model: sequences::ActiveModel = existing.into();
                        model.next_number = Set(next_number);
                        model.update(transaction)?;
                    } else {
                        sequences::ActiveModel {
                            year: Set(year),
                            next_number: Set(next_number),
                        }
                        .insert(transaction)?;
                    }
                }
                Ok(())
            })
            .map_err(|error| match error {
                sea_orm::TransactionError::Connection(error)
                | sea_orm::TransactionError::Transaction(error) => error,
            })
    }

    pub fn logo_bytes(&self) -> Result<Option<Vec<u8>>, DbErr> {
        invoice_profile::Entity::find_by_id(1)
            .one(self.db)
            .map(|row| row.and_then(|row| row.logo))
    }

    pub fn set_logo(&self, bytes: Vec<u8>, now: i64) -> Result<(), DbErr> {
        invoice_profile::Entity::update_many()
            .col_expr(invoice_profile::Column::Logo, Expr::value(bytes))
            .col_expr(invoice_profile::Column::LogoMime, Expr::value("image/png"))
            .col_expr(invoice_profile::Column::UpdatedAt, Expr::value(now))
            .filter(invoice_profile::Column::Id.eq(1))
            .exec(self.db)
            .map(|_| ())
    }

    pub fn clear_logo(&self, now: i64) -> Result<(), DbErr> {
        invoice_profile::Entity::update_many()
            .col_expr(invoice_profile::Column::Logo, Expr::value(None::<Vec<u8>>))
            .col_expr(
                invoice_profile::Column::LogoMime,
                Expr::value(None::<String>),
            )
            .col_expr(invoice_profile::Column::UpdatedAt, Expr::value(now))
            .filter(invoice_profile::Column::Id.eq(1))
            .exec(self.db)
            .map(|_| ())
    }

    pub fn highest_issued_number(&self, year: i64) -> Result<Option<i64>, DbErr> {
        let prefix = format!("INV-{year}-");
        let numbers = invoices::Entity::find()
            .filter(invoices::Column::Number.is_not_null())
            .all(self.db)?
            .into_iter()
            .filter_map(|row| row.number)
            .filter_map(|number| {
                number
                    .strip_prefix(&prefix)
                    .and_then(|value| value.parse().ok())
            })
            .collect::<Vec<i64>>();
        Ok(numbers.into_iter().max())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::entities::invoice_profile::{invoices, sequences};
    use crate::database::sea_orm::{ConnectionTrait, Database, DbBackend, Schema};

    #[test]
    fn profile_and_sequence_round_trip_without_raw_queries() {
        let db = Database::connect("sqlite::memory:").unwrap();
        let schema = Schema::new(DbBackend::Sqlite);
        db.execute(&schema.create_table_from_entity(invoice_profile::Entity))
            .unwrap();
        db.execute(&schema.create_table_from_entity(sequences::Entity))
            .unwrap();
        db.execute(&schema.create_table_from_entity(invoices::Entity))
            .unwrap();
        invoice_profile::Entity::insert(invoice_profile::ActiveModel {
            id: Set(1),
            name: Set("Aevio".into()),
            address: Set("Berlin".into()),
            email: Set(None),
            phone: Set(None),
            payment_instructions: Set(None),
            default_notes: Set(None),
            default_terms_days: Set(14),
            logo: Set(None),
            logo_mime: Set(None),
            updated_at: Set(1),
        })
        .exec(&db)
        .unwrap();
        invoices::Entity::insert(invoices::ActiveModel {
            id: Set("invoice".into()),
            number: Set(Some("INV-2026-0041".into())),
        })
        .exec(&db)
        .unwrap();
        let repository = InvoiceProfileRepository::new(&db);
        assert_eq!(repository.get(2026).unwrap().next_number, 1);
        assert_eq!(repository.highest_issued_number(2026).unwrap(), Some(41));
        repository
            .update(
                ProfileUpdate {
                    name: "Updated".into(),
                    address: "Hamburg".into(),
                    email: Some("billing@example.test".into()),
                    phone: None,
                    payment_instructions: None,
                    default_notes: None,
                    default_terms_days: 30,
                    next_number: Some(42),
                },
                2026,
                2,
            )
            .unwrap();
        assert_eq!(repository.get(2026).unwrap().next_number, 42);
        repository.set_logo(vec![1, 2, 3], 3).unwrap();
        assert_eq!(repository.logo_bytes().unwrap(), Some(vec![1, 2, 3]));
        repository.clear_logo(4).unwrap();
        assert_eq!(repository.logo_bytes().unwrap(), None);
    }
}
