use std::collections::HashMap;
use tui::text::{Span, Spans};
use tui::widgets::{ListItem, ListState, Block, Borders, BorderType, List};
use tui::style::{Style, Modifier, Color};
use std::fs::File;
use std::path::Path;
use std::error::Error;
use regex::Regex;
use log::debug;
use chrono::{DateTime, Utc};
use num_traits::{ FromPrimitive, ToPrimitive };
use num_derive::{ FromPrimitive, ToPrimitive };
use num;
use rusqlite::ToSql;
use crate::db;

#[derive(FromPrimitive, ToPrimitive)]
pub enum CompleteStatus {
    Incomplete = 1,
    Complete = 2,
}

#[derive(FromPrimitive, ToPrimitive)]
pub enum ExpandStatus{
    Closed = 1,
    Open = 2,
}


///Represent items on the rutudu list
pub struct Item {
    pub id: u32,
    pub title: String,
    pub entry: String,
    ///if None, this is on the root level
    pub parent_id: u32,
    pub child_ids: Vec<u32>,
    pub expand: ExpandStatus,
    pub complete: CompleteStatus,
}

impl Item {

    // pub fn new(title:String)->{
    //     Item{title, entry, status: ItemStatus::Undone, parent: Box::new(None)}
    // }

    ///constructor, no parent
    pub fn new(id: u32, title: &str, entry: &str) -> Item {
        // Item { title, entry, parent: Box::new(None) }
        Item{id,
            title: title.to_string(),
            entry: entry.to_string(),
            parent_id: 0,
            child_ids: Vec::new(),
            expand:ExpandStatus::Closed,
            complete: CompleteStatus::Incomplete,
        }
    }
    ///constructor, parent
    pub fn new_with_parent(title: String, entry: String, parent_id:u32) -> Item {
        // Item { title, entry, parent: Box::new(None) }
        Item{id:0,
            title,
            entry,
            parent_id,
            child_ids:Vec::new(),
            expand:ExpandStatus::Closed,
            complete: CompleteStatus::Incomplete,
        }
        //update parent's child ids?? fuck
    }
    ///yep, has a parent but may not be leaf
    pub fn new_child(id: u32, title: String, entry: String, parent: Item) -> Item {
        Item { id,
            title,
            entry,
            parent_id: parent.id.clone(),
            child_ids: Vec::new(),
            expand: ExpandStatus::Closed,
            complete: CompleteStatus::Incomplete }
    }

    ///Symbol to indicate if item is expanded or collapsed
    pub fn expansion_state_symbol(&self)->String{
        return match self.expand{
            ExpandStatus::Open => String::from("[-]"),
            ExpandStatus::Closed => String::from("[+]"),
        }

    }

    ///Return the item as text, either just the title,
    /// or the title and the entry, depending on expand status
    pub fn text(&self, item_no:usize) -> Vec<Spans> {
        let mut modifier = match self.complete  {
                CompleteStatus::Complete => Modifier::CROSSED_OUT,
                CompleteStatus::Incomplete => Modifier::empty(),
            };
        let mut content = vec![Spans::from(
            Span::styled(format!("{}: {} {}",
                              &item_no, &self.expansion_state_symbol(), self.title),
            Style::default().add_modifier(modifier)))];
        //show our expanded content if need be
        if let ExpandStatus::Open = self.expand{
            content.push(Spans::from(Span::raw(format!("    {}", self.entry))));
        }
        content
    }
}

///This will be a hierarchy of items, associated by parent id, until we get to the last one
pub struct StatefulMap<T>{
    pub items: HashMap<u32,StatefulList<T>>,
}

#[derive(Clone)]
pub struct StatefulList<T> {
    pub state: ListState,
    pub items: Vec<T>,
}

impl<T> StatefulList<T> {
    pub fn new() -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items: Vec::new(),
}
    }

    pub fn with_items(items: Vec<T>) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items,
        }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn unselect(&mut self) {
        self.state.select(None);
    }
}

#[derive(Eq, PartialEq)]
pub enum InputMode {
    Insert,
    InsertChild,
    Edit,
    Save,
    Open,
    Quit,
}

pub struct RutuduList {
    ///what mode are we in?
    pub input_mode: InputMode,
    pub items: StatefulList<Item>,
    pub open_file_dialog_files: StatefulList<String>,

    ///if the list has been saved, this is where
    pub file_path:String,
    ///When opening files, we only want to scan the one time
    pub has_scanned:bool,

    pub current_item: String,
    /// This is the x,y of the cursor
    pub cursor_position: [u16;2],

}

///New todolist out of nuffink
impl Default for RutuduList {
    fn default() -> Self {
        RutuduList {
            input_mode: InputMode::Edit,
            items: StatefulList::new(),
            open_file_dialog_files: StatefulList::new(),
            current_item: "".to_string(),
            cursor_position: [ 1,1 ],
            file_path: String::new(),
            has_scanned: false,
        }
    }
}

impl RutuduList {
    pub fn enter_edit_mode(&mut self) {
        self.cursor_position = [1, 1];
        self.input_mode = InputMode::Edit;
    }

    pub fn enter_save_mode(&mut self) {
        self.cursor_position = [1, 1];
        self.input_mode = InputMode::Save;
    }

    pub fn is_save_mode(&self)->bool{
        return self.input_mode == InputMode::Save;
    }

    pub fn enter_quit_mode(&mut self) {
        self.input_mode = InputMode::Quit;
    }
    //Create an item as a sub item of the currently selected item
    pub fn enter_child_insert_mode(&mut self) {
        // let i = self.items.state.selected().unwrap_or(0);
        self.input_mode = InputMode::InsertChild;
    }
    //Create an item at the current level
    pub fn enter_insert_mode(&mut self) {
        self.input_mode = InputMode::Insert;
    }

    pub fn enter_open_mode(&mut self){
        self.input_mode = InputMode::Open;
    }

    pub fn close_selected(&mut self){
        let i = self.items.state.selected().unwrap_or(0);
        self.items.items[i].expand = ExpandStatus::Closed;
    }

    pub fn expand_selected(&mut self){
        let i = self.items.state.selected().unwrap_or(0);
        self.items.items[i].expand = ExpandStatus::Open;
    }

    pub fn toggle_selected_status(&mut self){
        let i = self.items.state.selected().unwrap_or(0);

        self.items.items[i].complete = match self.items.items[i].complete{
            CompleteStatus::Incomplete => CompleteStatus::Complete,
            CompleteStatus::Complete => CompleteStatus::Incomplete,
        }
    }

    pub fn load_list_from_file_dialog(&mut self){
        let s = self.open_file_dialog_files.state.clone();
        let filename = self.open_file_dialog_files.items[s.selected().unwrap_or(0)].clone();
        match db::load_list( self, &filename){
            Ok(_) => {}
            Err(why) => panic!("Failed to load list {}", why),
        }
        self.enter_edit_mode();
    }

    pub fn open_file_up(&mut self){
        self.open_file_dialog_files.previous();
    }

    pub fn open_file_down(&mut self){
        self.open_file_dialog_files.next();
    }

    pub fn items_as_vec(&self)->Vec<ListItem>{
        let item_ref = &self.items;
            item_ref
            .items
            .iter()
            .enumerate()
            .map(|(i, msg)| {
                let mut content = msg.text(i);
                ListItem::new(content)
            }).collect()
    }

    pub fn down(&mut self){
        self.items.next();
    }

    pub fn up(&mut self){
        self.items.previous();
    }

    pub fn right(&mut self){
        if let Some(i) = self.items.state.selected(){
            self.items.items[i].expand = ExpandStatus::Open;
        }
    }

    pub fn left(&mut self){
        if let Some(i) = self.items.state.selected(){
            self.items.items[i].expand = ExpandStatus::Closed;
        }
    }

    pub fn add_item_to_list(&mut self){
        //todo here we will insert child if in the insertchild mode
        //it will use the currently selected node if exists or 0 otherwise
        //here we get the parent id if it exists
        let mut item = self.get_current_input_as_item();
        item.parent_id = if self.input_mode==InputMode::InsertChild {
            if let Some(i) = self.items.state.selected(){
                self.items.items[i].id.clone()
            }else{ 0 }
        }else{ 0 };

        self.items.items.push(item);
        self.enter_edit_mode();
    }
    pub fn get_current_input_as_item(&mut self) ->Item{
        let mut entry:String = self.current_item.drain(..).collect();
        //split by newlines
        let first_new_line = entry.find("\n").unwrap_or(entry.len());
        let title:String = entry.drain(..first_new_line).collect();
        // content - we set the id to the maximum
        Item::new(self.items.items.len().clone() as u32,&title, &entry)
    }

    ///Add character to current input
    /// while keeping track of the cursor
    pub fn add_character(&mut self, c:char){
        self.current_item.push(c);
        if c == '\n'{ //newline!
            self.cursor_position[0] = 1;
            self.cursor_position[1] = self.cursor_position[1]+1;
        }else{
            // print!("Goodbye");
            self.cursor_position[0]= self.cursor_position[0]+1;
        }
    }

    pub fn remove_character(&mut self){
        //do nothing if current_item is zero length
        if self.current_item.len()==0{
            return ();
        }
        let c = self.current_item.pop().unwrap_or('\0');
        if c == '\n' {//deleted a new line!
            //reduce y by 1
            self.cursor_position[1]=self.cursor_position[1]-1;
            //we need the len of this line.....!
            let mut line_len = 0;
            //find out length of line we are at the end of
            match self.current_item.rfind('\n') {
                None => line_len = self.current_item.len(),
                Some(nli) => line_len = self.current_item.len() - nli,
            }
            //put cursor at end of line
            self.cursor_position[0]=line_len as u16;
        }else{
            //reduce x by 1
            self.cursor_position[0]=self.cursor_position[0]-1;
        }
    }

    pub fn add_save_file_char(&mut self, c:char){
        self.file_path.push(c);//no need to check
        self.cursor_position[0]=self.cursor_position[0]+1;
    }

    pub fn remove_save_file_char(&mut self){
        self.file_path.pop();
        self.cursor_position[0] = self.cursor_position[0]-1;
    }

    pub fn list_name(&mut self)->String{
        //trim off the first path of the filepath`
        let fp = self.file_path.clone();
        match fp.rfind("/"){
           None => fp,
           Some(i) => fp.split_at(i+1).1.to_string(),//get the last part, eg foom.rtd from /home/foom/foom.rtd
        }
    }

    pub fn clear_file_list(&mut self){

    }

    ///
    /// This wil read all the '*rtd' file names and return them in result
    ///
    pub fn scan_directory(&mut self, dir_path:&str, extension:&str) -> Result<Vec<String>, Box<dyn Error>> {
        let mut lists = Vec::new();
        let search_str = format!(r"^.*\.{}$", extension);
        // debug!("Going to scan for {} on path {}", search_str, dir_path);
        // let rx = Regex::new(r".*rtd$")?;
        let rx = Regex::new(&search_str)?;
        //get the current directory,
        // let current_dir = std::env::current_dir()?;
        let dir = Path::new(&dir_path);
        for entry_result in std::fs::read_dir(dir)? {
            let entry = entry_result?;
            let path = entry.path();
            let path_str = path.into_os_string().into_string().unwrap();
            // debug!("Reading dir, found: {}",&path_str);
            if rx.is_match(&path_str) {
                // debug!("Matches!");
                lists.push(path_str);
            }
        }
        Ok(lists)
    }

    ///Will scan the current directory once, to prevent loop jamming
    pub fn scan_files_once(&mut self){
        if self.has_scanned{
           return;
        }
        // debug!("Scanning files...");
        //go through the directory
        let tudu_files = match self.scan_directory("./", "rtd"){
            Err(e) => panic!("Unable to open file dialog: {}", e),
            Ok(entries) => entries,
        };
        // debug!("We found {} files!",  &tudu_files.len());
        // tudu_files.i
         tudu_files.iter()
             .for_each(|s|{
             // debug!("Pushing {}", s);
           self.open_file_dialog_files.items.push(String::from(s));
        });
        self.has_scanned = true;
    }

    ///reset the scan variable
    pub fn reset_scan_guard(&mut self){
        self.has_scanned=false;
    }
}