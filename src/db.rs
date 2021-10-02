use std::error::Error;
use log::{debug, error};
use num_traits::{ToPrimitive, FromPrimitive};



use crate::model::{RutuduList, Item, CompleteStatus, ExpandStatus};
use rusqlite::{Connection, params};
use std::path::Path;

pub fn save_list(list: &RutuduList) -> Result<(), Box<dyn Error>> {
    debug!("About to save list, number of items: {}", list.items.items.len().to_string());
    let mut fp = &list.file_path.clone();
    let mut fp_suffixed = String::new();
    if !fp.to_ascii_lowercase().ends_with(".rtd") {
        fp_suffixed= format!("{}.rtd", fp);
    }
    let conn = Connection::open(fp_suffixed)?;
    create_table_if_needed(&conn);
    for item in &list.items.items{
        match conn.execute("INSERT INTO rutudu_list(parent_id, title, entry, create_date, completeStatus, expandStatus )
                                    VALUES(?1,?2,?3,strftime('%Y-%m-%d %H-%M-%S','now'), ?4, ?5 )", params![0, &item.title, &item.entry, &item.complete.to_u8(), &item.expand.to_u8()]){
            Ok(updated) => debug!("Number of rows inserted: {}", updated),
            Err(why) => error!("Failed to insert row: {}", why),
        }
    }
    // list.items
    //     .items
    //     .iter()
    //     .map(|i| {
    //         debug!("About to run the execute");
    //         match conn.execute("INSERT INTO rutudu_list(parent_id, title, entry, create_date)
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
            completeStatus SMALLINT,
            expandStatus SMALLINT,
            create_date DATE
        );
    ", []).unwrap();
}

///Load new list into our current list - gooodbye old list!
pub fn load_list(tudu_list: &mut RutuduList, file_name: &str) ->Result<(), Box<dyn Error>>{
    //save old one
    save_list(tudu_list)?;
    debug!("About to load new list");
    //import the new items into our list
    tudu_list.file_path = String::from(file_name);
    let conn = Connection::open(Path::new(file_name))?;
    let mut stmt = conn
        .prepare("select id, title, entry, parent_id, completeStatus, expandStatus from rutudu_list")?;

    //need to do child ids someho        Item { id: 0, title, entry, parent_id: parent.id.clone(), child_ids: Vec::new(), expand: ExpandStatus::Closed, complete: CompleteStatus::Incomplete }w
    let item_iter = stmt.query_map([],|row|{
        Ok(Item{
            id: row.get("id")?,
            title: row.get("title")?,
            entry: row.get("entry")?,
            parent_id: row.get("parent_id")?,
            child_ids: Vec::new(),
            complete: FromPrimitive::from_u8(row.get("completeStatus")?).unwrap_or(CompleteStatus::Incomplete),
            expand: FromPrimitive::from_u8(row.get("expandStatus")?).unwrap_or(ExpandStatus::Closed),
            })
        })?;

    tudu_list.items.items.clear();
    for item in item_iter {
        tudu_list.items.items.push(item.unwrap());
    }

    Ok(())
}
