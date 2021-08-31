use std::error::Error;
use log::debug;


use crate::model::RutuduList;
use rusqlite::{Connection, params};

pub fn save_list(list: &RutuduList) -> Result<(), Box<dyn Error>> {
    debug!("About to save list...");
    let conn = Connection::open(&list.file_path)?;
    create_table_if_needed(&conn);
    list.items
        .items
        .iter()
        .map(|i| {
            conn.execute("INSERT INTO rutudu_list(parent_id, title, entry, createDate) \
                                    VALUES(?1,?2,?3,now()", params![0, &i.title, &i.entry]);
        });
    Ok(())
}

pub fn create_table_if_needed(conn: &Connection) {
    conn.execute("
        CREATE TABLE IF NOT EXISTS rutudu_list(
            id INTEGER PRIMARY KEY ASC,
            parent_id INTEGER,
            title TEXT NOT NULL,
            entry TEXT,
            createDate DATE
        );
    ", []);
}