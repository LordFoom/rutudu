use std::error::Error;
use log::{debug, error};
use num_traits::{ToPrimitive, FromPrimitive};



use crate::model::{RutuduList, Item, CompleteStatus, ExpandStatus};
use rusqlite::{Connection, params};
use std::path::Path;
#[cfg(feature="clockrust")]
use clockrusting::db::ClockRuster;

pub fn save_list(list: &RutuduList) -> Result<(), Box<dyn Error>> {
    let fp = &list.file_path();
    debug!("About to save list '{}', number of items: {}", fp, list.items.items.len().to_string());
    let fp_suffixed= if !fp.to_ascii_lowercase().ends_with(".rtd") {
        format!("{}.rtd", fp)
    }else{
        fp.to_string()
    };
    debug!("Connection will be file: {}", fp_suffixed);
    let conn = Connection::open(fp_suffixed)?;
    create_table_if_needed(&conn);
    match empty_table(&conn){
       Ok(_) => debug!("Emptied table successfully"),
        Err(e) => error!("Could not empty table? {}", e),
    }
    list.item_tree.iter()
        .for_each(|(_, sub_list )|{
           sub_list.iter().for_each(|item|{
               debug!("Trying to insert '{}' item with '{}' id", item.title, item.id);
               match conn.execute("INSERT INTO rutudu_list(id, parent_id, title, entry, completeStatus, expandStatus , create_date)
                                    VALUES(?1, ?2, ?3, ?4, ?5, ?6, strftime('%Y-%m-%d %H-%M-%S','now') )",
                                  params![&item.id, &item.parent_id, &item.title, &item.entry, &item.complete.to_u8(), &item.expand.to_u8()]){
                   Ok(updated) => debug!("Number of rows inserted: {}", updated),
                   Err(why) => error!("Failed to insert row: {}", why),

           }})
        });
    Ok(())
}

pub  fn empty_table(conn: &Connection)->Result<(), Box<dyn Error>>{
    conn.execute_batch(r" DELETE FROM rutudu_list;
                             VACUUM;")?;
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
    //save old one -- no, there may not be one
    // save_list(tudu_list)?;
    debug!("About to load new list");
    //import the new items into our list
    tudu_list.set_file_path(file_name);
    let conn = Connection::open(Path::new(file_name))?;
    let mut stmt = conn
        .prepare("select id, title, entry, parent_id, completeStatus, expandStatus from rutudu_list")?;

    //need to do child ids someho        Item { id: 0, title, entry, parent_id: parent.id.clone(), child_ids: Vec::new(), expand: ExpandStatus::Closed, complete: CompleteStatus::Incomplete }
    let mut items:Vec<Item> = stmt.query_map([],|row|{
        Ok(Item{
            id: row.get("id")?,
            title: row.get("title")?,
            entry: row.get("entry")?,
            parent_id: row.get("parent_id")?,
            child_ids: Vec::new(),
            complete: FromPrimitive::from_u8(row.get("completeStatus")?).unwrap_or(CompleteStatus::Incomplete),
            expand: FromPrimitive::from_u8(row.get("expandStatus")?).unwrap_or(ExpandStatus::Closed),
            depth:0,
            order:0,
            tracking_time: false,
            })
        })?
        .map(|i| i.unwrap()).collect();

    tudu_list.item_tree.clear();
    //don't need to clear the list
    // tudu_list.items.items.clear();

    items.iter_mut()
        .for_each(|i|{
            i.tracking_time = false;
            #[cfg(feature="clockrust")]{
                let cr = ClockRuster::init(&tudu_list.file_path);
                i.tracking_time = match cr.currently_tracking(&i.title){
                    Ok(y) => {
                        debug!("We got a result for currently tracking: {}, which was: {} ", &i.title, y);
                        y
                    },
                    Err(why) =>{
                        error!("Error when trying to check tracking: {}, reason: {}",&i.title, why);
                        false
                    },
                }
            };
            // check_currently_being_tracked(i);
            tudu_list.insert_item(i);
        });
    //only need to do this once
    // tudu_list.dirty_list = true;

    Ok(())
}