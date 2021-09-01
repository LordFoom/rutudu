use std::error::Error;
use log::{debug, error};


use crate::model::RutuduList;
use rusqlite::{Connection, params};

pub fn save_list(list: &RutuduList) -> Result<(), Box<dyn Error>> {
    debug!("About to save list, number of items: {}", list.items.items.len().to_string());
    let conn = Connection::open(&list.file_path)?;
    create_table_if_needed(&conn);
    for item in &list.items.items{
        match conn.execute("INSERT INTO rutudu_list(parent_id, title, entry, createDate)
                                    VALUES(?1,?2,?3,strftime('%Y-%m-%d %H-%M-%S','now') )", params![0, &item.title, &item.entry]){
            Ok(updated) => debug!("Number of rows inserted: {}", updated),
            Err(why) => error!("Failed to insert row: {}", why),
        }
    }
    // list.items
    //     .items
    //     .iter()
    //     .map(|i| {
    //         debug!("About to run the execute");
    //         match conn.execute("INSERT INTO rutudu_list(parent_id, title, entry, createDate)
    //                                 VALUES(?1,?2,?3,now()", params![0, &i.title, &i.entry]){
    //             Ok(updated) => debug!("Number of rows inserted: {}", updated),
    //             Err(why) => error!("Failed to insert row: {}", why),
    //         }
    //     });
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
    ", []).unwrap();
}