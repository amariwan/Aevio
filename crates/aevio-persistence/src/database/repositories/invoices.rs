use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, DbErr, EntityTrait,
    QueryFilter, QueryOrder, TransactionError,
};

use crate::database::entities::invoice_profile::sequences;
use crate::database::entities::invoices::{
    clients, entries, lines, projects, records, records as invoices,
};

pub struct InvoiceRepository<'db> {
    db: &'db DatabaseConnection,
}

#[derive(Debug, Clone)]
pub struct BillableRow {
    pub entry: entries::Model,
    pub project: projects::Model,
    pub rate: Option<f64>,
    pub on_invoice: bool,
}

#[derive(Debug, Clone)]
pub struct DraftRecord {
    pub id: String,
    pub client_id: String,
    pub client_name: String,
    pub client_email: Option<String>,
    pub client_address: Option<String>,
    pub issue_date: String,
    pub due_date: String,
    pub terms_days: i64,
    pub subject: Option<String>,
    pub notes: Option<String>,
    pub payment_instructions: Option<String>,
    pub from_name: String,
    pub from_address: String,
    pub from_email: Option<String>,
    pub from_phone: Option<String>,
    pub total_cents: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct DraftLine {
    pub id: String,
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

#[derive(Debug, Clone)]
pub struct TimeLineSource {
    pub project_name: String,
    pub started_at: i64,
    pub ended_at: i64,
    pub description: String,
    pub rate: Option<f64>,
}

impl<'db> InvoiceRepository<'db> {
    pub fn new(db: &'db DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn list(&self) -> Result<Vec<invoices::Model>, DbErr> {
        let mut rows = invoices::Entity::find().all(self.db)?;
        rows.sort_by(|left, right| {
            right
                .issued_at
                .or(Some(right.created_at))
                .cmp(&left.issued_at.or(Some(left.created_at)))
                .then_with(|| right.id.cmp(&left.id))
        });
        Ok(rows)
    }

    pub fn get(&self, id: &str) -> Result<Option<(invoices::Model, Vec<lines::Model>)>, DbErr> {
        let Some(invoice) = invoices::Entity::find_by_id(id.to_owned()).one(self.db)? else {
            return Ok(None);
        };
        let lines = lines::Entity::find()
            .filter(lines::Column::InvoiceId.eq(id.to_owned()))
            .order_by_asc(lines::Column::Position)
            .all(self.db)?;
        Ok(Some((invoice, lines)))
    }

    pub fn billable(
        &self,
        client_id: &str,
        start: i64,
        end: i64,
        invoice_id: Option<&str>,
    ) -> Result<Vec<BillableRow>, DbErr> {
        let clients = clients::Entity::find_by_id(client_id.to_owned())
            .one(self.db)?
            .filter(|row| row.deleted_at.is_none());
        let Some(client) = clients else {
            return Ok(Vec::new());
        };
        let projects = projects::Entity::find()
            .filter(projects::Column::ClientId.eq(client_id.to_owned()))
            .filter(projects::Column::DeletedAt.is_null())
            .all(self.db)?;
        let project_ids = projects
            .iter()
            .map(|row| row.id.clone())
            .collect::<Vec<_>>();
        if project_ids.is_empty() {
            return Ok(Vec::new());
        }
        let entries = entries::Entity::find()
            .filter(entries::Column::ProjectId.is_in(project_ids))
            .filter(entries::Column::Status.eq("approved"))
            .filter(entries::Column::Billable.eq(1))
            .filter(entries::Column::DeletedAt.is_null())
            .filter(entries::Column::StartedAt.gte(start))
            .filter(entries::Column::StartedAt.lt(end))
            .order_by_asc(entries::Column::StartedAt)
            .order_by_asc(entries::Column::Id)
            .all(self.db)?;
        let by_project = projects
            .into_iter()
            .map(|row| (row.id.clone(), row))
            .collect::<std::collections::HashMap<_, _>>();
        Ok(entries
            .into_iter()
            .filter(|row| {
                row.invoice_id
                    .as_deref()
                    .is_none_or(|id| Some(id) == invoice_id)
            })
            .filter_map(|entry| {
                let project_id = entry.project_id.as_ref()?;
                let project = by_project.get(project_id)?.clone();
                Some(BillableRow {
                    rate: project.hourly_rate.or(client.default_rate),
                    on_invoice: entry.invoice_id.is_some(),
                    entry,
                    project,
                })
            })
            .collect::<Vec<_>>())
    }

    pub fn pdf(&self, id: &str) -> Result<Option<Vec<u8>>, DbErr> {
        invoices::Entity::find_by_id(id.to_owned())
            .one(self.db)
            .map(|row| row.and_then(|row| row.pdf))
    }

    pub fn active_client(&self, id: &str) -> Result<bool, DbErr> {
        Ok(clients::Entity::find_by_id(id.to_owned())
            .one(self.db)?
            .is_some_and(|row| row.deleted_at.is_none()))
    }

    pub fn time_line_source(
        &self,
        entry_id: &str,
        client_id: &str,
        invoice_id: Option<&str>,
    ) -> Result<Option<TimeLineSource>, DbErr> {
        let Some(entry) = entries::Entity::find_by_id(entry_id.to_owned()).one(self.db)? else {
            return Ok(None);
        };
        if entry.status != "approved"
            || entry.billable != 1
            || entry.deleted_at.is_some()
            || (entry.invoice_id.is_some() && entry.invoice_id.as_deref() != invoice_id)
        {
            return Ok(None);
        }
        let Some(project_id) = entry.project_id.as_deref() else {
            return Ok(None);
        };
        let Some(project) = projects::Entity::find_by_id(project_id.to_owned())
            .one(self.db)?
            .filter(|row| row.deleted_at.is_none())
        else {
            return Ok(None);
        };
        let Some(client) = clients::Entity::find_by_id(client_id.to_owned())
            .one(self.db)?
            .filter(|row| row.deleted_at.is_none() && row.id == client_id)
        else {
            return Ok(None);
        };
        if project.client_id.as_deref() != Some(client_id) {
            return Ok(None);
        }
        Ok(Some(TimeLineSource {
            project_name: project.name,
            started_at: entry.started_at,
            ended_at: entry.ended_at,
            description: entry.description,
            rate: project.hourly_rate.or(client.default_rate),
        }))
    }

    pub fn save_draft(&self, record: DraftRecord, lines: &[DraftLine]) -> Result<(), DbErr> {
        self.db
            .transaction::<_, _, DbErr>(|transaction| {
                if let Some(existing) =
                    records::Entity::find_by_id(record.id.clone()).one(transaction)?
                {
                    if existing.status != "draft" {
                        return Err(DbErr::Custom("Only drafts can be edited".into()));
                    }
                    let mut model: records::ActiveModel = existing.into();
                    model.client_id = Set(record.client_id.clone());
                    model.client_name = Set(record.client_name.clone());
                    model.client_email = Set(record.client_email.clone());
                    model.client_address = Set(record.client_address.clone());
                    model.issue_date = Set(Some(record.issue_date.clone()));
                    model.due_date = Set(Some(record.due_date.clone()));
                    model.terms_days = Set(Some(record.terms_days));
                    model.subject = Set(record.subject.clone());
                    model.notes = Set(record.notes.clone());
                    model.payment_instructions = Set(record.payment_instructions.clone());
                    model.from_name = Set(record.from_name.clone());
                    model.from_address = Set(record.from_address.clone());
                    model.from_email = Set(record.from_email.clone());
                    model.from_phone = Set(record.from_phone.clone());
                    model.total_cents = Set(record.total_cents);
                    model.updated_at = Set(record.updated_at);
                    model.update(transaction)?;
                } else {
                    records::ActiveModel {
                        id: Set(record.id.clone()),
                        number: Set(None),
                        client_id: Set(record.client_id.clone()),
                        client_name: Set(record.client_name.clone()),
                        client_email: Set(record.client_email.clone()),
                        client_address: Set(record.client_address.clone()),
                        currency: Set("USD".into()),
                        status: Set("draft".into()),
                        issue_date: Set(Some(record.issue_date.clone())),
                        due_date: Set(Some(record.due_date.clone())),
                        terms_days: Set(Some(record.terms_days)),
                        subject: Set(record.subject.clone()),
                        notes: Set(record.notes.clone()),
                        payment_instructions: Set(record.payment_instructions.clone()),
                        from_name: Set(record.from_name.clone()),
                        from_address: Set(record.from_address.clone()),
                        from_email: Set(record.from_email.clone()),
                        from_phone: Set(record.from_phone.clone()),
                        total_cents: Set(record.total_cents),
                        pdf: Set(None),
                        issued_at: Set(None),
                        paid_at: Set(None),
                        created_at: Set(record.updated_at),
                        updated_at: Set(record.updated_at),
                    }
                    .insert(transaction)?;
                }
                entries::Entity::update_many()
                    .col_expr(
                        entries::Column::InvoiceId,
                        sea_orm::sea_query::Expr::value(None::<String>),
                    )
                    .filter(entries::Column::InvoiceId.eq(record.id.clone()))
                    .exec(transaction)?;
                lines::Entity::delete_many()
                    .filter(lines::Column::InvoiceId.eq(record.id.clone()))
                    .exec(transaction)?;
                for line in lines {
                    lines::ActiveModel {
                        id: Set(line.id.clone()),
                        invoice_id: Set(record.id.clone()),
                        position: Set(line.position),
                        kind: Set(line.kind.clone()),
                        entry_id: Set(line.entry_id.clone()),
                        project_name: Set(line.project_name.clone()),
                        description: Set(line.description.clone()),
                        started_at: Set(line.started_at),
                        ended_at: Set(line.ended_at),
                        quantity_hundredths: Set(line.quantity_hundredths),
                        unit: Set(line.unit.clone()),
                        rate_cents: Set(line.rate_cents),
                        amount_cents: Set(line.amount_cents),
                    }
                    .insert(transaction)?;
                    if let Some(entry_id) = &line.entry_id {
                        let updated = entries::Entity::update_many()
                            .col_expr(
                                entries::Column::InvoiceId,
                                sea_orm::sea_query::Expr::value(record.id.clone()),
                            )
                            .filter(entries::Column::Id.eq(entry_id.clone()))
                            .filter(entries::Column::InvoiceId.is_null())
                            .filter(entries::Column::Status.eq("approved"))
                            .filter(entries::Column::Billable.eq(1))
                            .filter(entries::Column::DeletedAt.is_null())
                            .exec(transaction)?;
                        if updated.rows_affected != 1 {
                            return Err(DbErr::Custom(
                                "Time changed while saving the invoice".into(),
                            ));
                        }
                    }
                }
                Ok(())
            })
            .map_err(|error| match error {
                TransactionError::Connection(error) | TransactionError::Transaction(error) => error,
            })
    }

    pub fn set_paid(&self, id: &str, paid: bool, now: i64) -> Result<(), DbErr> {
        let (from, to) = if paid {
            ("open", "paid")
        } else {
            ("paid", "open")
        };
        let result = records::Entity::update_many()
            .col_expr(records::Column::Status, sea_orm::sea_query::Expr::value(to))
            .col_expr(
                records::Column::PaidAt,
                sea_orm::sea_query::Expr::value(paid.then_some(now)),
            )
            .col_expr(
                records::Column::UpdatedAt,
                sea_orm::sea_query::Expr::value(now),
            )
            .filter(records::Column::Id.eq(id.to_owned()))
            .filter(records::Column::Status.eq(from))
            .exec(self.db)?;
        (result.rows_affected == 1)
            .then_some(())
            .ok_or_else(|| DbErr::Custom(format!("Invoice is not {from}")))
    }

    pub fn void(&self, id: &str, now: i64) -> Result<(), DbErr> {
        self.db
            .transaction::<_, _, DbErr>(|transaction| {
                let result = invoices::Entity::update_many()
                    .col_expr(
                        invoices::Column::Status,
                        sea_orm::sea_query::Expr::value("void"),
                    )
                    .col_expr(
                        invoices::Column::UpdatedAt,
                        sea_orm::sea_query::Expr::value(now),
                    )
                    .filter(invoices::Column::Id.eq(id.to_owned()))
                    .filter(invoices::Column::Status.eq("open"))
                    .exec(transaction)?;
                if result.rows_affected != 1 {
                    return Err(DbErr::Custom(
                        "Only open invoices can be voided; mark a paid invoice unpaid first".into(),
                    ));
                }
                entries::Entity::update_many()
                    .col_expr(
                        entries::Column::InvoiceId,
                        sea_orm::sea_query::Expr::value(None::<String>),
                    )
                    .filter(entries::Column::InvoiceId.eq(id.to_owned()))
                    .exec(transaction)?;
                lines::Entity::update_many()
                    .col_expr(
                        lines::Column::EntryId,
                        sea_orm::sea_query::Expr::value(None::<String>),
                    )
                    .filter(lines::Column::InvoiceId.eq(id.to_owned()))
                    .exec(transaction)?;
                Ok(())
            })
            .map_err(|error| match error {
                TransactionError::Connection(error) | TransactionError::Transaction(error) => error,
            })
    }

    pub fn delete_draft(&self, id: &str) -> Result<(), DbErr> {
        self.delete_by_status(id, "draft", "Only draft invoices can be deleted")
    }

    pub fn delete_void(&self, id: &str) -> Result<(), DbErr> {
        self.delete_by_status(id, "void", "Only void invoices can be deleted")
    }

    pub fn finalize(
        &self,
        id: &str,
        number: String,
        next_number: i64,
        pdf: Vec<u8>,
        now: i64,
        year: i64,
    ) -> Result<(), DbErr> {
        self.db
            .transaction::<_, _, DbErr>(|transaction| {
                let result = invoices::Entity::update_many()
                    .col_expr(
                        invoices::Column::Status,
                        sea_orm::sea_query::Expr::value("open"),
                    )
                    .col_expr(
                        invoices::Column::Number,
                        sea_orm::sea_query::Expr::value(number.clone()),
                    )
                    .col_expr(
                        invoices::Column::Pdf,
                        sea_orm::sea_query::Expr::value(pdf.clone()),
                    )
                    .col_expr(
                        invoices::Column::IssuedAt,
                        sea_orm::sea_query::Expr::value(now),
                    )
                    .col_expr(
                        invoices::Column::UpdatedAt,
                        sea_orm::sea_query::Expr::value(now),
                    )
                    .filter(invoices::Column::Id.eq(id.to_owned()))
                    .filter(invoices::Column::Status.eq("draft"))
                    .exec(transaction)?;
                if result.rows_affected != 1 {
                    return Err(DbErr::Custom("Only drafts can be finalized".into()));
                }
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
                Ok(())
            })
            .map_err(|error| match error {
                TransactionError::Connection(error) | TransactionError::Transaction(error) => error,
            })
    }

    fn delete_by_status(&self, id: &str, status: &str, message: &str) -> Result<(), DbErr> {
        self.db
            .transaction::<_, _, DbErr>(|transaction| {
                let Some(invoice) = invoices::Entity::find_by_id(id.to_owned()).one(transaction)?
                else {
                    return Err(DbErr::Custom("Invoice not found".into()));
                };
                if invoice.status != status {
                    return Err(DbErr::Custom(message.into()));
                }
                entries::Entity::update_many()
                    .col_expr(
                        entries::Column::InvoiceId,
                        sea_orm::sea_query::Expr::value(None::<String>),
                    )
                    .filter(entries::Column::InvoiceId.eq(id.to_owned()))
                    .exec(transaction)?;
                lines::Entity::delete_many()
                    .filter(lines::Column::InvoiceId.eq(id.to_owned()))
                    .exec(transaction)?;
                invoices::Entity::delete_by_id(id.to_owned()).exec(transaction)?;
                Ok(())
            })
            .map_err(|error| match error {
                TransactionError::Connection(error) | TransactionError::Transaction(error) => error,
            })
    }
}
