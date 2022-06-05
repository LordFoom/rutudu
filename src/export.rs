use std::collections::HashMap;
use std::error::Error;
use std::fs;
use log::debug;
use crate::model::Item;

pub fn write_list_as_markdown(list_name: &str, list: &HashMap<u32, Vec<Item>>, file_path:&str)->Result<(), Box<dyn Error>>{
    debug!("Writing markdown to {}...", file_path);
    //create the contents to write
    let mut contents= format!("## {}\n", list_name);
    contents.push_str(&list_into_string(list, 0, 0)?);
    fs::write(file_path, contents)?;
    Ok(())
}


///Starting with the 0 idx, will recursively write out each item and its children into the string
pub fn list_into_string(list: &HashMap<u32, Vec<Item>>, list_idx: u32, depth: u16) ->Result<String, Box<dyn Error>>{
    debug!("Writing at depth: {}", depth);
    let mut contents = String::new();
    //get the sublist
    //get the list using the id
    // let sl = list.entry(list_idx)?;
    if let Some(mut sl) = list.get(&list_idx) {
        sl.into_iter()
          .enumerate()
          .for_each(|(i, item)| {
              let indent = str::repeat(" ", (depth*4).into());
              contents.push_str(&format!("{}{}. ",indent, i+1));
              contents.push_str(&item.to_string());
              if list.contains_key(&item.id){
                  let sub_string = list_into_string(list, item.id, depth+1).unwrap();
                  contents.push_str(&sub_string)
              }
          });
    }
    //create a string with 4xdepth spaces in front and appropriately numbered
    //if an item has children, call this method on the children before continuing

    Ok(contents)
}