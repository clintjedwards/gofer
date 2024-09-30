use crate::storage::{map_rusqlite_error, StorageError};
use sea_query::{Expr, Iden, Query, SqliteQueryBuilder};
use sea_query_rusqlite::RusqliteBinder;
use tokio_rusqlite::{Connection, Row};

#[derive(Clone, Debug, Default)]
pub struct Token {
    pub id: String,
    pub hash: String,
    pub created: String,
    pub metadata: String,
    pub expires: String,
    pub disabled: bool,
    pub roles: String,
    pub user: String,
}

impl From<&Row<'_>> for Token {
    fn from(row: &Row) -> Self {
        Self {
            id: row.get_unwrap("id"),
            hash: row.get_unwrap("hash"),
            created: row.get_unwrap("created"),
            metadata: row.get_unwrap("metadata"),
            expires: row.get_unwrap("expires"),
            disabled: row.get_unwrap("disabled"),
            roles: row.get_unwrap("roles"),
            user: row.get_unwrap("user"),
        }
    }
}

#[derive(Iden)]
enum TokenTable {
    Table,
    Id,
    Hash,
    Created,
    Metadata,
    Expires,
    Disabled,
    Roles,
    User,
}

#[derive(Clone, Debug)]
pub struct UpdatableFields {
    pub disabled: Option<bool>,
}

pub fn insert(conn: &Connection, token: &Token) -> Result<(), StorageError> {
    let (sql, values) = Query::insert()
        .into_table(TokenTable::Table)
        .columns([
            TokenTable::Id,
            TokenTable::Hash,
            TokenTable::Created,
            TokenTable::Metadata,
            TokenTable::Expires,
            TokenTable::Disabled,
            TokenTable::User,
            TokenTable::Roles,
        ])
        .values_panic([
            token.id.clone().into(),
            token.hash.clone().into(),
            token.created.clone().into(),
            token.metadata.clone().into(),
            token.expires.clone().into(),
            token.disabled.into(),
            token.user.clone().into(),
            token.roles.clone().into(),
        ])
        .build_rusqlite(SqliteQueryBuilder);

    conn.execute(sql.as_str(), &*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    Ok(())
}

pub fn list(conn: &Connection) -> Result<Vec<Token>, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            TokenTable::Id,
            TokenTable::Hash,
            TokenTable::Created,
            TokenTable::Metadata,
            TokenTable::Expires,
            TokenTable::Disabled,
            TokenTable::User,
            TokenTable::Roles,
        ])
        .from(TokenTable::Table)
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut objects: Vec<Token> = vec![];

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        objects.push(Token::from(row));
    }

    Ok(objects)
}

pub fn get_by_id(conn: &Connection, id: &str) -> Result<Token, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            TokenTable::Id,
            TokenTable::Hash,
            TokenTable::Created,
            TokenTable::Metadata,
            TokenTable::Expires,
            TokenTable::Disabled,
            TokenTable::User,
            TokenTable::Roles,
        ])
        .from(TokenTable::Table)
        .and_where(Expr::col(TokenTable::Id).eq(id))
        .limit(1)
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        return Ok(Token::from(row));
    }

    Err(StorageError::NotFound)
}

pub fn get_by_hash(conn: &Connection, hash: &str) -> Result<Token, StorageError> {
    let (sql, values) = Query::select()
        .columns([
            TokenTable::Id,
            TokenTable::Hash,
            TokenTable::Created,
            TokenTable::Metadata,
            TokenTable::Expires,
            TokenTable::Disabled,
            TokenTable::User,
            TokenTable::Roles,
        ])
        .from(TokenTable::Table)
        .and_where(Expr::col(TokenTable::Hash).eq(hash))
        .limit(1)
        .build_rusqlite(SqliteQueryBuilder);

    let mut statement = conn
        .prepare(sql.as_str())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    let mut rows = statement
        .query(&*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    while let Some(row) = rows.next().map_err(|e| map_rusqlite_error(e, &sql))? {
        return Ok(Token::from(row));
    }

    Err(StorageError::NotFound)
}

pub fn update(conn: &Connection, id: &str, fields: UpdatableFields) -> Result<(), StorageError> {
    let mut query = Query::update();
    query.table(TokenTable::Table);

    if let Some(value) = fields.disabled {
        query.value(TokenTable::Disabled, value);
    }

    if query.get_values().is_empty() {
        return Err(StorageError::NoFieldsUpdated);
    }

    query.and_where(Expr::col(TokenTable::Id).eq(id));
    let (sql, values) = query.build_rusqlite(SqliteQueryBuilder);

    conn.execute(sql.as_str(), &*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> Result<(), StorageError> {
    let (sql, values) = Query::delete()
        .from_table(TokenTable::Table)
        .and_where(Expr::col(TokenTable::Id).eq(id))
        .build_rusqlite(SqliteQueryBuilder);

    conn.execute(sql.as_str(), &*values.as_params())
        .map_err(|e| map_rusqlite_error(e, &sql))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{tests::TestHarness, Connection};

    fn setup() -> Result<(TestHarness, Connection), Box<dyn std::error::Error>> {
        let harness = TestHarness::new();
        let mut conn = harness.write_conn().unwrap();

        let token = Token {
            id: "some_id".into(),
            hash: "some_hash".into(),
            created: "some_time".into(),
            metadata: "some_json_hashmap".into(),
            expires: "some_expiry".into(),
            user: "some_user".into(),
            roles: "{some_role_scheme}".into(),
            disabled: false,
        };

        insert(&mut conn, &token)?;

        Ok((harness, conn))
    }

    fn test_list_tokens() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let tokens = list(&mut conn).expect("Failed to list tokens");

        // Assert that we got at least one token back
        assert!(!tokens.is_empty(), "No tokens returned");

        for token in tokens {
            match token.id.as_str() {
                "some_id" => {
                    assert_eq!(token.hash, "some_hash");
                    assert_eq!(token.metadata, "some_json_hashmap");
                }
                _ => panic!("Unexpected token"),
            }
        }
    }

    fn test_update_tokens() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let fields_to_update = UpdatableFields {
            disabled: Some(true),
        };

        update(&mut conn, "some_id", fields_to_update).expect("Failed to update token");

        let updated_token =
            get_by_id(&mut conn, "some_id").expect("Failed to retrieve updated token");

        assert!(updated_token.disabled);
    }

    fn test_insert_and_get() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        let fetched_token = get_by_id(&mut conn, "some_id").expect("Failed to get Token");
        assert_eq!(fetched_token.id, "some_id");
        assert_eq!(fetched_token.roles, "{some_role_scheme}",);
    }

    fn test_delete() {
        let (_harness, mut conn) = setup().expect("Failed to set up DB");

        delete(&mut conn, "some_id").expect("Failed to delete Token");

        let result = get_by_id(&mut conn, "some_id");
        assert!(matches!(result, Err(StorageError::NotFound)));
    }
}
