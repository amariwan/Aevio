//! Workspace application models. Persistence is delegated to the typed
//! infrastructure repository; this module contains only mapping and policy.

use regex::Regex;
use sea_orm::ActiveValue::Set;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use aevio_persistence::database::entities::workspace::{
    attachments, comments, cycles, documents, initiatives, mentions, notes,
};
use aevio_persistence::database::repositories::workspace::WorkspaceRepository;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub id: String,
    pub title: String,
    pub content: String,
    pub created_at: u64,
    pub updated_at: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub title: String,
    pub body: String,
    pub created_at: u64,
    pub updated_at: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    pub id: String,
    pub target_type: String,
    pub target_id: String,
    pub body: String,
    pub author: String,
    pub created_at: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub id: String,
    pub target_type: String,
    pub target_id: String,
    pub name: String,
    pub path: String,
    pub mime_type: Option<String>,
    pub size: Option<u64>,
    pub created_at: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mention {
    pub id: String,
    pub target_type: String,
    pub target_id: String,
    pub handle: String,
    pub created_at: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cycle {
    pub id: String,
    pub name: String,
    pub goal: String,
    pub starts_at: u64,
    pub ends_at: u64,
    pub status: String,
    pub created_at: u64,
    pub updated_at: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Initiative {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub target_date: Option<u64>,
    pub project_ids: Vec<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewCycle {
    pub name: String,
    pub goal: String,
    pub starts_at: u64,
    pub ends_at: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewInitiative {
    pub name: String,
    pub description: String,
    pub target_date: Option<u64>,
    pub project_ids: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewDocument {
    pub title: String,
    pub content: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDocument {
    pub title: Option<String>,
    pub content: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewNote {
    pub title: String,
    pub body: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNote {
    pub title: Option<String>,
    pub body: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewComment {
    pub target_type: String,
    pub target_id: String,
    pub body: String,
    pub author: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewAttachment {
    pub target_type: String,
    pub target_id: String,
    pub name: String,
    pub path: String,
    pub mime_type: Option<String>,
    pub size: Option<u64>,
}

type Result<T> = std::result::Result<T, String>;
fn id() -> String {
    Uuid::now_v7().to_string()
}
fn db(error: sea_orm::DbErr) -> String {
    error.to_string()
}
fn missing(kind: &str, id: &str) -> String {
    format!("no {kind} with id {id}")
}

pub fn list_documents(db: &sea_orm::DatabaseConnection) -> Result<Vec<Document>> {
    WorkspaceRepository::new(db)
        .list_documents()
        .map_err(crate::workspace::db)
        .map(|rows| rows.into_iter().map(document).collect())
}
pub fn create_document(
    db: &sea_orm::DatabaseConnection,
    input: NewDocument,
    now: u64,
) -> Result<Document> {
    let id = id();
    WorkspaceRepository::new(db)
        .insert_document(documents::ActiveModel {
            id: Set(id.clone()),
            title: Set(input.title.trim().to_owned()),
            content: Set(input.content),
            created_at: Set(now as i64),
            updated_at: Set(now as i64),
        })
        .map_err(crate::workspace::db)?;
    get_document(db, &id)
}
pub fn update_document(
    db: &sea_orm::DatabaseConnection,
    id: &str,
    patch: UpdateDocument,
    now: u64,
) -> Result<Document> {
    let current = WorkspaceRepository::new(db)
        .document(id)
        .map_err(crate::workspace::db)?
        .ok_or_else(|| missing("document", id))?;
    WorkspaceRepository::new(db)
        .update_document(documents::ActiveModel {
            id: Set(id.to_owned()),
            title: Set(patch.title.unwrap_or(current.title)),
            content: Set(patch.content.unwrap_or(current.content)),
            created_at: Set(current.created_at),
            updated_at: Set(now as i64),
        })
        .map_err(crate::workspace::db)?;
    get_document(db, id)
}
pub fn delete_document(db: &sea_orm::DatabaseConnection, id: &str) -> Result<()> {
    WorkspaceRepository::new(db)
        .delete_document(id)
        .map_err(crate::workspace::db)
}
pub fn get_document(db: &sea_orm::DatabaseConnection, id: &str) -> Result<Document> {
    WorkspaceRepository::new(db)
        .document(id)
        .map_err(crate::workspace::db)?
        .map(document)
        .ok_or_else(|| missing("document", id))
}

pub fn list_notes(db: &sea_orm::DatabaseConnection) -> Result<Vec<Note>> {
    WorkspaceRepository::new(db)
        .list_notes()
        .map_err(crate::workspace::db)
        .map(|rows| rows.into_iter().map(note).collect())
}
pub fn create_note(db: &sea_orm::DatabaseConnection, input: NewNote, now: u64) -> Result<Note> {
    let id = id();
    WorkspaceRepository::new(db)
        .insert_note(notes::ActiveModel {
            id: Set(id.clone()),
            title: Set(input.title.trim().to_owned()),
            body: Set(input.body),
            created_at: Set(now as i64),
            updated_at: Set(now as i64),
        })
        .map_err(crate::workspace::db)?;
    get_note(db, &id)
}
pub fn update_note(
    db: &sea_orm::DatabaseConnection,
    id: &str,
    patch: UpdateNote,
    now: u64,
) -> Result<Note> {
    let current = WorkspaceRepository::new(db)
        .note(id)
        .map_err(crate::workspace::db)?
        .ok_or_else(|| missing("note", id))?;
    WorkspaceRepository::new(db)
        .update_note(notes::ActiveModel {
            id: Set(id.to_owned()),
            title: Set(patch.title.unwrap_or(current.title)),
            body: Set(patch.body.unwrap_or(current.body)),
            created_at: Set(current.created_at),
            updated_at: Set(now as i64),
        })
        .map_err(crate::workspace::db)?;
    get_note(db, id)
}
pub fn delete_note(db: &sea_orm::DatabaseConnection, id: &str) -> Result<()> {
    WorkspaceRepository::new(db)
        .delete_note(id)
        .map_err(crate::workspace::db)
}
pub fn get_note(db: &sea_orm::DatabaseConnection, id: &str) -> Result<Note> {
    WorkspaceRepository::new(db)
        .note(id)
        .map_err(crate::workspace::db)?
        .map(note)
        .ok_or_else(|| missing("note", id))
}

pub fn list_comments(
    db: &sea_orm::DatabaseConnection,
    target_type: &str,
    target_id: &str,
) -> Result<Vec<Comment>> {
    WorkspaceRepository::new(db)
        .list_comments(target_type, target_id)
        .map_err(crate::workspace::db)
        .map(|rows| rows.into_iter().map(comment).collect())
}
pub fn create_comment(
    db: &sea_orm::DatabaseConnection,
    input: NewComment,
    now: u64,
) -> Result<Comment> {
    let id = id();
    let author = input.author.unwrap_or_else(|| "You".to_owned());
    WorkspaceRepository::new(db)
        .insert_comment(comments::ActiveModel {
            id: Set(id.clone()),
            target_type: Set(input.target_type.clone()),
            target_id: Set(input.target_id.clone()),
            body: Set(input.body.clone()),
            author: Set(author),
            created_at: Set(now as i64),
        })
        .map_err(crate::workspace::db)?;
    let pattern = Regex::new(r"@([A-Za-z0-9_.-]+)").map_err(|error| error.to_string())?;
    let mentions = pattern
        .captures_iter(&input.body)
        .map(|capture| mentions::ActiveModel {
            id: Set(crate::workspace::id()),
            target_type: Set(input.target_type.clone()),
            target_id: Set(input.target_id.clone()),
            handle: Set(capture[1].to_owned()),
            created_at: Set(now as i64),
        })
        .collect();
    WorkspaceRepository::new(db)
        .replace_mentions(&input.target_type, &input.target_id, mentions)
        .map_err(crate::workspace::db)?;
    WorkspaceRepository::new(db)
        .comment(&id)
        .map_err(crate::workspace::db)?
        .map(comment)
        .ok_or_else(|| missing("comment", &id))
}
pub fn list_attachments(
    db: &sea_orm::DatabaseConnection,
    target_type: &str,
    target_id: &str,
) -> Result<Vec<Attachment>> {
    WorkspaceRepository::new(db)
        .list_attachments(target_type, target_id)
        .map_err(crate::workspace::db)
        .map(|rows| rows.into_iter().map(attachment).collect())
}
pub fn create_attachment(
    db: &sea_orm::DatabaseConnection,
    input: NewAttachment,
    now: u64,
) -> Result<Attachment> {
    let id = id();
    WorkspaceRepository::new(db)
        .insert_attachment(attachments::ActiveModel {
            id: Set(id.clone()),
            target_type: Set(input.target_type),
            target_id: Set(input.target_id),
            name: Set(input.name),
            path: Set(input.path),
            mime_type: Set(input.mime_type),
            size: Set(input.size.map(|value| value as i64)),
            created_at: Set(now as i64),
        })
        .map_err(crate::workspace::db)?;
    WorkspaceRepository::new(db)
        .attachment(&id)
        .map_err(crate::workspace::db)?
        .map(attachment)
        .ok_or_else(|| missing("attachment", &id))
}
pub fn list_mentions(
    db: &sea_orm::DatabaseConnection,
    target_type: &str,
    target_id: &str,
) -> Result<Vec<Mention>> {
    WorkspaceRepository::new(db)
        .list_mentions(target_type, target_id)
        .map_err(crate::workspace::db)
        .map(|rows| rows.into_iter().map(mention).collect())
}

pub fn list_cycles(db: &sea_orm::DatabaseConnection) -> Result<Vec<Cycle>> {
    WorkspaceRepository::new(db)
        .list_cycles()
        .map_err(crate::workspace::db)
        .map(|rows| rows.into_iter().map(cycle).collect())
}
pub fn create_cycle(db: &sea_orm::DatabaseConnection, input: NewCycle, now: u64) -> Result<Cycle> {
    let id = id();
    WorkspaceRepository::new(db)
        .insert_cycle(cycles::ActiveModel {
            id: Set(id.clone()),
            name: Set(input.name.trim().to_owned()),
            goal: Set(input.goal),
            starts_at: Set(input.starts_at as i64),
            ends_at: Set(input.ends_at as i64),
            status: Set("planned".to_owned()),
            created_at: Set(now as i64),
            updated_at: Set(now as i64),
        })
        .map_err(crate::workspace::db)?;
    WorkspaceRepository::new(db)
        .cycle(&id)
        .map_err(crate::workspace::db)?
        .map(cycle)
        .ok_or_else(|| missing("cycle", &id))
}
pub fn list_initiatives(db: &sea_orm::DatabaseConnection) -> Result<Vec<Initiative>> {
    WorkspaceRepository::new(db)
        .list_initiatives()
        .map_err(crate::workspace::db)
        .map(|rows| {
            rows.into_iter()
                .map(|(row, projects)| initiative(row, projects))
                .collect()
        })
}
pub fn create_initiative(
    db: &sea_orm::DatabaseConnection,
    input: NewInitiative,
    now: u64,
) -> Result<Initiative> {
    let id = id();
    WorkspaceRepository::new(db)
        .insert_initiative(
            &id,
            initiatives::ActiveModel {
                id: Set(id.clone()),
                name: Set(input.name.trim().to_owned()),
                description: Set(input.description),
                status: Set("active".to_owned()),
                target_date: Set(input.target_date.map(|value| value as i64)),
                created_at: Set(now as i64),
                updated_at: Set(now as i64),
            },
            &input.project_ids,
        )
        .map_err(crate::workspace::db)?;
    list_initiatives(db)?
        .into_iter()
        .find(|row| row.id == id)
        .ok_or_else(|| "initiative was not created".to_owned())
}

fn document(row: documents::Model) -> Document {
    Document {
        id: row.id,
        title: row.title,
        content: row.content,
        created_at: row.created_at as u64,
        updated_at: row.updated_at as u64,
    }
}
fn note(row: notes::Model) -> Note {
    Note {
        id: row.id,
        title: row.title,
        body: row.body,
        created_at: row.created_at as u64,
        updated_at: row.updated_at as u64,
    }
}
fn comment(row: comments::Model) -> Comment {
    Comment {
        id: row.id,
        target_type: row.target_type,
        target_id: row.target_id,
        body: row.body,
        author: row.author,
        created_at: row.created_at as u64,
    }
}
fn attachment(row: attachments::Model) -> Attachment {
    Attachment {
        id: row.id,
        target_type: row.target_type,
        target_id: row.target_id,
        name: row.name,
        path: row.path,
        mime_type: row.mime_type,
        size: row.size.map(|value| value as u64),
        created_at: row.created_at as u64,
    }
}
fn mention(row: mentions::Model) -> Mention {
    Mention {
        id: row.id,
        target_type: row.target_type,
        target_id: row.target_id,
        handle: row.handle,
        created_at: row.created_at as u64,
    }
}
fn cycle(row: cycles::Model) -> Cycle {
    Cycle {
        id: row.id,
        name: row.name,
        goal: row.goal,
        starts_at: row.starts_at as u64,
        ends_at: row.ends_at as u64,
        status: row.status,
        created_at: row.created_at as u64,
        updated_at: row.updated_at as u64,
    }
}
fn initiative(row: initiatives::Model, project_ids: Vec<String>) -> Initiative {
    Initiative {
        id: row.id,
        name: row.name,
        description: row.description,
        status: row.status,
        target_date: row.target_date.map(|value| value as u64),
        project_ids,
        created_at: row.created_at as u64,
        updated_at: row.updated_at as u64,
    }
}
