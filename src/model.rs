use std::borrow::Borrow;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::mem;
use std::os::unix::process::parent_id;
use std::path::Path;

use chrono::{DateTime, Utc};
use log::{debug, warn, error};
use num;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};
use regex::Regex;
use rusqlite::ToSql;
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, Borders, BorderType, List, ListItem, ListState};

use crate::{db, show_new_item_input};

#[derive(FromPrimitive, ToPrimitive, Clone)]
pub enum CompleteStatus {
    Incomplete = 1,
    Complete = 2,
}

#[derive(FromPrimitive, ToPrimitive, Clone, PartialEq, PartialOrd, Debug)]
pub enum ExpandStatus {
    Closed = 1,
    ShowChildren = 2,
    Open = 3,
}


///Represent items on the rutudu list
#[derive(Clone)]
pub struct Item {
    pub id: u32,
    pub title: String,
    pub entry: String,
    ///if None, this is on the root level
    pub parent_id: u32,
    pub child_ids: Vec<u32>,
    pub expand: ExpandStatus,
    pub complete: CompleteStatus,
    pub depth: usize,
}

impl Item {

    // pub fn new(title:String)->{
    //     Item{title, entry, status: ItemStatus::Undone, parent: Box::new(None)}
    // }

    ///constructor, no parent
    pub fn new(id: u32, title: &str, entry: &str) -> Item {
        // Item { title, entry, parent: Box::new(None) }
        Item {
            id,
            title: title.to_string(),
            entry: entry.to_string(),
            parent_id: 0,
            child_ids: Vec::new(),
            expand: ExpandStatus::Closed,
            complete: CompleteStatus::Incomplete,
            depth: 0,
        }
    }
    ///constructor, parent
    pub fn new_with_parent(rutudu_list: RutuduList, title: String, entry: String, parent_id: u32) -> Item {
        // Item { title, entry, parent: Box::new(None) }
        Item {
            id: 0,
            title,
            entry,
            parent_id,
            child_ids: Vec::new(),
            expand: ExpandStatus::Closed,
            complete: CompleteStatus::Incomplete,
            depth: 0,//probably need to do something like that
        }
        //update parent's child ids?? fuck
    }
    ///yep, has a parent but may not be leaf
    pub fn new_child(rutudu_list: RutuduList, id: u32, title: String, entry: String, parent: Item) -> Item {
        Item {
            id,
            title,
            entry,
            parent_id: parent.id.clone(),
            child_ids: Vec::new(),
            expand: ExpandStatus::Closed,
            complete: CompleteStatus::Incomplete,
            depth: parent.depth + 1,

        }
    }

    ///Symbol to indicate if item is expanded or collapsed
    pub fn expansion_state_symbol(&self) -> String {
        return match self.expand {
            ExpandStatus::Open => String::from("[-]"),
            ExpandStatus::Closed => String::from("[+]"),
            ExpandStatus::ShowChildren => String::from("[|]"),
        };

    }

    ///todo Is THIS where we decide if and how to display children
    ///Return the item as text, either just the title,
    /// or the title and the entry, depending on expand status
    pub fn text(&self, item_no: usize) -> Vec<Spans> {
        let mut modifier = match self.complete {
            CompleteStatus::Complete => Modifier::CROSSED_OUT,
            CompleteStatus::Incomplete => Modifier::empty(),
        };
        let depth_string = "--".to_string().repeat(self.depth);
        let mut content = vec![Spans::from(
            Span::styled(format!("{}{}.{}: {} {}", depth_string,
                                 &item_no, &self.depth, &self.expansion_state_symbol(), self.title),
                         Style::default().add_modifier(modifier)))];
        //show our expanded content if need be
        if let ExpandStatus::Open = self.expand {
            content.push(Spans::from(Span::raw(format!("    {}", self.entry))));
        }
        content
    }

    //Increase expansion status from closed to show children to open
    pub fn expand(&mut self) {
        debug!("Hello");
        match self.expand {
            ExpandStatus::Closed => {
                self.expand = ExpandStatus::ShowChildren
            }
            ExpandStatus::ShowChildren => self.expand = ExpandStatus::Open,
            ExpandStatus::Open => {}//do nothing
        };
        debug!("Expand status: {:?}",self.expand);
    }

    pub fn show_children(&mut self){
        if self.expand==ExpandStatus::Closed {
            self.expand = ExpandStatus::ShowChildren;
        }
    }

    ///Decrease expansion status from open to show children to closed
    pub fn collapse(&mut self) {
        match self.expand {
            ExpandStatus::Closed => {}//do nothing
            ExpandStatus::ShowChildren => self.expand = ExpandStatus::Closed,
            ExpandStatus::Open => self.expand = ExpandStatus::ShowChildren,
        };
    }

    pub fn toggle_complete_status(&mut self) {
        self.complete = match self.complete {
            CompleteStatus::Incomplete => CompleteStatus::Complete,
            CompleteStatus::Complete => CompleteStatus::Incomplete,
        }
    }

    #[test]
    pub fn test_collapse() {}

    // pub fn depth(&self)->u8{
    //     if self.parent_id == 0{
    //         return 0;
    //     }
    //     // return 1 + get_parent().depth();
    //     1
    // }

    pub fn should_show_children(&self) -> bool {
        self.expand >= ExpandStatus::ShowChildren
    }

    pub fn is_collapsed(&self) -> bool {
        self.expand == ExpandStatus::Closed
    }

    pub fn is_open(&self) -> bool {
        self.expand == ExpandStatus::Open
    }

    pub fn is_closed(&self) -> bool {
        self.is_collapsed()
    }
    // pub fn get_parent() -> Option<Item>{
    //
    // }
}

pub struct MapState {
    /// for root items==0, then id 1 has a list of its children at index 1, id 2 at idx 2, and so forth
    item_idx: usize,
    offset: usize,
    ///row and column
    selected: Option<(usize, usize)>,
}

impl Default for MapState {
    fn default() -> MapState {
        MapState {
            item_idx: 0,
            offset: 0,
            selected: None,
        }
    }
}

impl MapState {}

///This will be a hierarchy of items, associated by parent id, until we get to the last one
// #[derive(Clone)]
// pub struct StatefulMap<T>{
//     pub state: MapState,
//     ///This maps the parent_id to a stateful list
//     pub items: HashMap<u32,StatefulList<T>>,
// }
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


    pub fn with_items_and_state(items: Vec<T>, state: ListState) -> StatefulList<T> {
        StatefulList {
            state,
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
        if !self.items.is_empty() {
            self.state.select(Some(i));
        }
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
        if !self.items.is_empty() {
            self.state.select(Some(i));
        }
    }

    pub fn unselect(&mut self) {
        self.state.select(None);
    }
}

#[derive(Eq, PartialEq, Clone)]
pub enum InputMode {
    Insert,
    InsertChild,
    InsertParent,
    Edit,
    Save,
    Open,
    Quit,
}

#[derive(Clone)]
pub struct RutuduList {
    ///what mode are we in?
    pub input_mode: InputMode,
    pub items: StatefulList<Item>,
    ///Here we keep our tree
    /// those without a parent, root, in 0
    /// those WITH
    pub item_tree: HashMap<u32, Vec<Item>>,
    pub open_file_dialog_files: StatefulList<String>,

    ///if the list has been saved, this is where
    pub file_path: String,
    ///When opening files, we only want to scan the one time
    pub has_scanned: bool,

    pub current_item: String,
    /// This is the x,y of the cursor
    pub cursor_position: [u16; 2],
    /// This tells us if we need to rebuild the list
    pub dirty_list: bool,
    /// This tells us if a list has unsaved changes
    pub unsaved: bool,

}

///New todolist out of nuffink
impl Default for RutuduList {
    fn default() -> Self {
        RutuduList {
            input_mode: InputMode::Edit,
            items: StatefulList::new(),
            item_tree: HashMap::new(),
            open_file_dialog_files: StatefulList::new(),
            current_item: "".to_string(),
            cursor_position: [1, 1],
            file_path: String::new(),
            has_scanned: false,
            dirty_list: false,
            unsaved: false,
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

    pub fn mark_saved(&mut self){
        self.unsaved = false;
        self.enter_edit_mode();
    }

    pub fn is_save_mode(&self) -> bool {
        return self.input_mode == InputMode::Save;
    }

    pub fn enter_quit_mode(&mut self) {
        self.input_mode = InputMode::Quit;
    }
    //Create an item as a sub item of the currently selected item
    pub fn enter_child_insert_mode(&mut self) {
        debug!("Insert child mode");
        // let i = self.items.state.selected().unwrap_or(0);
        self.input_mode = InputMode::InsertChild;
    }
    pub fn enter_parent_insert_mode(&mut self) {
        debug!("Insert parent mode");
        // let i = self.items.state.selected().unwrap_or(0);
        self.input_mode = InputMode::InsertParent;
    }
    //Create an item at the current level
    pub fn enter_insert_mode(&mut self) {
        self.input_mode = InputMode::Insert;
    }

    pub fn enter_open_mode(&mut self) {
        self.input_mode = InputMode::Open;
    }

    pub fn collapse_selected(&mut self) {
        let i = self.items.state.selected().unwrap_or(0);
        //expand the parent
        //get the parent id and then get the item and set its expansion status
        let parent_id = self.items.items[i].parent_id;
        debug!("Parent id:{}", parent_id);
        let item_id = self.items.items[i].id;
        if let Some(root) = self.item_tree.get_mut(&parent_id) {//get the list we belong to, could be zero
            root.iter_mut().for_each(|item| {
                if item.id == item_id {
                    item.collapse();
                }
            })
        };
        self.dirty_list = true;
    }

    ///Moves expansion status up the scale
    // pub fn expand_selected(&mut self){
    //     debug!("Expanding selected");
    //     let i = self.items.state.selected().unwrap_or(0);
    //     //expand the parent
    //     debug!("Expanding parent");
    //     self.items.items[i].expand();
    //     //get the parent id and then get the item and set its expansion status
    //     let parent_id = self.items.items[i].parent_id;
    //     let item_id = self.items.items[i].id;
    //     debug!("Expanding children");
    //     if parent_id > 0 {
    //         if let Some(children) =  self.item_tree.get_mut(&parent_id) {
    //             children.iter_mut().for_each(|item|{
    //                 item.expand();
    //                 // if item.id == item_id {
    //                 //     item.expand();
    //                 // }
    //             })}};
    //     self.dirty_list = true;
    // }

    ///Moves expansion status up the scale
    pub fn expand_selected(&mut self) {
        let i = self.items.state.selected().unwrap_or(0);
        //expand the parent
        // self.items.items[i].expand(); // this doesn't matter because we are rebuilding the list from the map
        //get the parent id and then get the item and set its expansion status
        let parent_id = self.items.items[i].parent_id;
        let item_id = self.items.items[i].id;

        self.item_tree
            .entry(parent_id)
            .or_insert_with(Vec::new)
            .iter_mut()
            .filter(|i| i.id == item_id)
            .for_each(|i| i.expand());
        // .for_each(|item| {
        // if item.id == item_id {//should only happen once
        //     item.expand();
        // }
        //
        // });
        // if let Some(item_list) = self.item_tree.get_mut(&parent_id) {
        //     item_list.iter_mut().for_each(|item|{
        //         if item.id == item_id {//should only happen once
        //             item.expand();
        //             //if it has no children, immediately expand again, don't need to display kids
        //             //     let expand_twice = if let Some(kids) = self.item_tree.get(&item.id){
        //             //         if kids.len() == 0 {
        //             //             true
        //             //         }
        //             //         else {
        //             //             false
        //             //         }
        //             //     } else {
        //             //         true
        //             //     };
        //             //
        //             //     if expand_twice {
        //             //         item.expand();
        //             //     }
        //             // }
        //             // mem::replace(self.item_tree.)
        //         }})}
        self.dirty_list = true;
    }

    // pub fn expand_subtree(&mut self, parent_id:&u32){
    //     //get the vec i belong to
    //     if let Some(elems) = self.item_tree.get_mut(parent_id) {
    //         for item in elems {
    //             item.expand();
    //             self.expand_subtree(&item.id);
    //         }
    //     }
    // }


    pub fn toggle_selected_status(&mut self) {
        let i = self.items.state.selected().unwrap_or(0);
        //mark it on the list
        if let Some(item) = self.items.items.get_mut(i) {
            item.toggle_complete_status();
            //mark it on the tree
            if let Some(container_vec_) = self.item_tree.get_mut(&item.parent_id) {
                container_vec_.iter_mut()
                              .filter(|i| &i.id == &item.id)
                              .for_each(|i| i.toggle_complete_status());
            }
        } else {
            warn!("Tried to toggle complete status with nothing selected")
        };
    }

    pub fn clear_list(&mut self) {
        self.items.items.clear();
    }

    pub fn load_list_from_file_dialog(&mut self) {
        let s = self.open_file_dialog_files.state.clone();
        let filename = self.open_file_dialog_files.items[s.selected().unwrap_or(0)].clone();
        match db::load_list(self, &filename) {
            Ok(_) => {}
            Err(why) => panic!("Failed to load list {}", why),
        }
        self.enter_edit_mode();
    }

    pub fn open_file_up(&mut self) {
        self.open_file_dialog_files.previous();
    }

    pub fn open_file_down(&mut self) {
        self.open_file_dialog_files.next();
    }
    pub fn items_as_vec(&self) -> Vec<ListItem> {
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

    // pub fn items_as_vec(&self)->StatefulList<ListItem>{
    //     // debug!("Items as vec");
    //     //we go over the root nodes
    //     let mut item_list = StatefulList::new();
    //     let mut calced_state= self.items.state.clone();
    //     let item_vec = if let Some(root_list) = self.item_tree.get(&0){
    //         // self.clear_list();
    //         root_list
    //             .iter()
    //             .enumerate()
    //             .flat_map(|(i, msg)| {
    //                 // self.build_subtree(msg, i,0, &calced_state )
    //                  ListItem::new(msg)
    //             }).collect()
    //     }else{
    //         Vec::new()
    //     };
    //
    //     item_list.items = item_vec;
    //     item_list.state = calced_state;
    //     item_list
    // }

    ///If the list is dirty, we create a new one from the hashmap
    pub fn rebuild_list_if_dirty(&mut self) {
        if self.dirty_list {
            self.rebuild_list();
        }
    }
    pub fn rebuild_list(&mut self) {
        // debug!("Building the stateful list");
        self.dirty_list = false;
        //get the map and the list of the root - our forest!
        if self.item_tree.is_empty() {//if there is no root list, nothing to do
            return;
        }

        let root_items_vec = self.get_subtree_vec(0, 0);
        self.items.items.clear();
        let list_items: StatefulList<ListItem> = StatefulList::new();
        root_items_vec.iter().enumerate().for_each(|(i, item)| {
            let new_item = ListItem::new(item.text(i));
            self.items.items.push(item.clone());
        });
        //now build a stateful list
        // mem::replace(self.items, list);
        //for each item, build a subtree
    }

    pub fn get_subtree_vec(&self, parent_id: u32, depth: usize) -> Vec<Item> {
        let mut ret_list = Vec::new();
        if self.item_tree.contains_key(&parent_id) {
            let item_subtree_vec: Vec<Item> = self.item_tree[&parent_id].clone();
            for mut item in item_subtree_vec {
                item.depth = depth;
                ret_list.push(item.clone());
                if item.should_show_children() {
                    let sub_sub_tree_vec = self.get_subtree_vec(item.id.clone(), depth + 1);
                    sub_sub_tree_vec.iter().for_each(|i| { ret_list.push(i.clone()) })
                }
            }
        }
        return ret_list;
    }

    // ///Help build the spans of our subtree - will also make sure the list has all the right items in it
    // pub fn build_subtree<'a>(&'a self, item:&'a Item, item_no:usize, depth:usize, calced_state:&ListState) -> Vec<ListItem<'a>> {
    //     debug!("We're building the subtree for item {} ", item.id);
    //     //we need to in here do the item itself, man!
    //     let mut item_text_as_vec:Vec<Spans<'a>> = item.text(item_no.clone(), self.depth);
    //     let mut vec_list:Vec<ListItem<'a>> = Vec::new();
    //     let li = ListItem::new(item_text_as_vec);
    //     vec_list.push(li);
    //     if item.is_collapsed(){
    //         vec_list
    //     }else{
    //         Vec::new()
    //     }
    //     // debug!("Build subtree for item: {}", item.id.to_string());
    //     // if let Some(children) = self.item_tree.get(&item.id) {
    //     //     //now we get the children
    //     //     for child in children {
    //     //         &self.build_subtree(child, item_no.clone(),depth.clone() + 1, calced_state)
    //     //         // for span in sub_vec {
    //     //         //     item_text_as_vec.push(span.clone());
    //     //         // }
    //     //     };
    //     //     item_text_as_vec
    //     // }else{
    //     //     Vec::new()
    //     // }
    // }

    pub fn build_item_spans_as_vec(&self, item_no: usize, item_id: u32, depth: usize) -> Vec<Spans> {
        // let item = self.items.items.get(item_id as usize).unwrap();
        let item = self.items.items.get(item_id as usize).unwrap();
        // let mut item_text_as_vec= item.text(item_no, depth);
        let mut item_text_as_vec = item.text(item_no.clone());
        //now we get the children
        if let Some(children) = self.item_tree.get(&item_id) {
            let i = 0;
            for child in children {
                // let sub_number = format!("{}.{}", item_no, i);
                let sub_vec = &self.build_item_spans_as_vec(item_no.clone(), child.id.clone(), depth.clone() + 1);
                for span in sub_vec {
                    item_text_as_vec.push(span.clone());
                }
                // item_text_as_vec.append(&mut sub_vec);
            }
            // children.items.iter()
            //         .cloned()
            //         .enumerate()
            //         .for_each(|(i, child)|{
            //             let sub_number = format!("{}.{}", item_no, i);
            //             let mut vec:Vec<Spans<'a>> = self.build_item_spans_as_vec(child.id.clone(), &sub_number.clone(), depth.clone() + 1);
            //             item_text_as_vec.append(&mut vec);
            //         });
        };
        // debug!("About to return {}", item_text_as_vec);
        item_text_as_vec
    }

    // pub fn get_item_and_child_text<'a>(&'a mut self, item:&'a Item, item_no:&'a str, depth:usize)->Vec<Spans<'a>>{
    //    let mut content = item.text(item_no, depth);
    //     if let Some(children) = self.item_tree.get(&item.id){
    //        let child_content = children.items
    //            .iter()
    //            .enumerate()
    //            .flat_map(|( i,child )|{
    //                let sub_number = format!("{}.{}", item_no, i);
    //                let child_spans = self.get_item_and_child_text(&child.clone(), &sub_number, depth + 1);
    //                // child_spans.iter().for_each(|cs|  { content.push(cs.clone()) });
    //                child_spans
    //            }).collect::<Vec<Spans>>();
    //            // .for_each(|(i, child)|{
    //            //     let sub_number = format!("{}.{}", item_no, i);
    //            //     let child_spans = self.get_item_and_child_text(child, &sub_number, depth + 1);
    //            //     child_spans.iter().for_each(|cs|  { content.push(cs.clone()) });
    //            //     // content.append(&child_spans);
    //            // });
    //     }
    //     content
    // }

    pub fn down(&mut self) {
        self.items.next();
    }

    pub fn up(&mut self) {
        self.items.previous();
    }

    pub fn right(&mut self) {
        if let Some(i) = self.items.state.selected() {
            self.items.items[i].expand();
        }
    }

    pub fn left(&mut self) {
        if let Some(i) = self.items.state.selected() {
            self.items.items[i].collapse();
        }
    }


    pub fn add_item_to_list(&mut self) {
        // debug!("Adding item to list");
        //it will use the currently selected node if exists or 0 otherwise
        //here we get the parent id if it exists
        let mut item = self.get_current_input_as_item();
        //get the parents id if it is in insertchild mode
        self.add_item(&mut item);

        //and we also keep a list of all the items

        self.enter_edit_mode();
    }

    ///Add item as parent to currently selected item, or, if noe, just add it


    pub fn add_item(&mut self, item: &mut Item) {
        debug!("Adding item, id: {} ", item.id);
        if item.parent_id == 0 {
            item.parent_id = match self.input_mode {
                InputMode::InsertChild =>{
                    if let Some(i) = self.items.state.selected() {
                        debug!("Found selected index: {}", i);
                        //we want to update the children ids as well as parent ids
                        let parent_id = self.items.items[i].id.clone();
                        debug!("Discovered parent id: {}", parent_id);
                        //children lists are by implication - mapped by item.id in the hashmap
                        parent_id
                    }else { 0 }
                }
                InputMode::InsertParent =>{
                    //we make it it's own parent, after insertion we're going to swap it
                    //with the selected item
                    item.id
                }
                _ =>  {0}
            }
        }//this will leave parent ids intact, which is good for when we open up lists
        debug!("Parent id: {}", item.parent_id.clone());

        //now we place the item in the right bucket
        let mut new_item = item.clone();
         self.item_tree
            .entry(item.parent_id.clone())
            .or_insert_with(Vec::new)
            .push(new_item);

        //now we do postprocessing
        if self.input_mode == InputMode::InsertParent {
            self.add_item_as_parent(item)
        } else if self.input_mode == InputMode::InsertChild {
            let opt_parent = match self.items.state.selected() {
                Some(i) => self.items.items.get_mut(i),
                None => None,
            };
            if let Some(list_item) = opt_parent{
                self.item_tree.entry(list_item.parent_id.clone())
                    .or_insert_with(Vec::new)
                    .iter_mut()
                    .filter(|i| { i.id == list_item.id })
                    .for_each(|i| { i.show_children()})

            }
        }

        self.dirty_list = true;
        self.unsaved = true;
    }

    ///Will add item as the parent of the currently selected item, if one is selected
    fn add_item_as_parent(&mut self, item: &mut Item) {
        let opt_parent = self.selected_item();

        //if we add as new parent, we now SWAP the two around in the tree :D
        if let Some(parent) = opt_parent {
            let old_parent_id = parent.parent_id.clone();
            let old_id = parent.id.clone();
            let new_item_id = item.id.clone();

            let mut old_item = None;
            let mut new_item = None;
            for (parent_id, mut children) in self.item_tree.iter_mut() {
                if parent_id == &old_parent_id {
                    for child in children.iter_mut() {
                        if child.id == old_id {
                            child.parent_id = new_item_id;
                            old_item = Some(child);
                        }
                    }
                } else if parent_id == &new_item_id {
                    for child in children.iter_mut() {
                        if child.id == new_item_id {
                            child.parent_id = old_parent_id;
                            //we want to see the old parent under the new parent
                            child.expand = ExpandStatus::ShowChildren;
                            new_item = Some(child);
                        }
                    }
                }
            }

            match old_item {
                Some(item_one) => {
                    match new_item {
                        Some(item_two) => {
                            mem::swap(item_one, item_two);//we should have place the item in its own subtree, ie it was its own parent but not in 0 until now
                        }
                        None => error!("Failed to find new item when adding new parent")
                    }
                }
                None => error!("Failed to find old item when adding parent!")
            }
        }
    }

    fn selected_item(&mut self) -> Option<&mut Item> {
        let opt_parent = match self.items.state.selected() {
            Some(i) => self.items.items.get_mut(i),
            None => None,
        };
        opt_parent
    }


    pub fn get_current_input_as_item(&mut self) -> Item {
        let mut entry: String = self.current_item.drain(..).collect();
        //split by newlines
        let first_new_line = entry.find("\n").unwrap_or(entry.len());
        let title: String = entry.drain(..first_new_line).collect();
        // content - we set the id to the maximum
        //we set the content to the number of TOTAL items
        let total_length:usize = self.item_tree.iter()
            .map(|(k, v)|  v.len() )
            .sum();
        debug!("Next item id: {}", total_length+1);
        // let i = self.items.items.len().clone() as u32;
        //we want to start this at ONE so we reserve the zero index for the root nodes of the forest
        Item::new((total_length as u32)+1, &title, &entry)
    }

    ///Add character to current input
    /// while keeping track of the cursor
    pub fn add_character(&mut self, c: char) {
        self.current_item.push(c);
        if c == '\n' { //newline!
            self.cursor_position[0] = 1;
            self.cursor_position[1] = self.cursor_position[1] + 1;
        } else {
            self.cursor_position[0] = self.cursor_position[0] + 1;
        }
    }

    pub fn remove_character(&mut self) {
        //do nothing if current_item is zero length
        if self.current_item.len() == 0 {
            return ();
        }
        let c = self.current_item.pop().unwrap_or('\0');
        if c == '\n' {//deleted a new line!
            //reduce y by 1
            self.cursor_position[1] = self.cursor_position[1] - 1;
            //we need the len of this line.....!
            let mut line_len = 0;
            //find out length of line we are at the end of
            match self.current_item.rfind('\n') {
                None => line_len = self.current_item.len(),
                Some(nli) => line_len = self.current_item.len() - nli,
            }
            //put cursor at end of line
            self.cursor_position[0] = line_len as u16;
        } else {
            //reduce x by 1
            self.cursor_position[0] = self.cursor_position[0] - 1;
        }
    }

    pub fn add_save_file_char(&mut self, c: char) {
        self.file_path.push(c);//no need to check
        self.cursor_position[0] = self.cursor_position[0] + 1;
    }

    pub fn remove_save_file_char(&mut self) {
        if self.file_path.len() == 0 {
            return;
        }
        debug!("remove_save_file_char, where x={}, y={} ", self.cursor_position[0], self.cursor_position[1]);

        self.file_path.pop();
        // self.cursor_position[0] = self.cursor_position[0]-1;
        // self.cursor_position[0] = self.file_path.len() as u16;
        debug!("x={}, y={} ", self.cursor_position[0], self.cursor_position[1]);
    }

    pub fn list_name(&mut self) -> String {
        //we add an asterisk if it is unsaved
        let save_needed = if self.unsaved { "*"} else{ ""};
        let fp = format!("{}{}", self.file_path.clone(), save_needed);
        //trim off the first path of the filepath`
        match fp.rfind("/") {
            None => fp,
            Some(i) => fp.split_at(i + 1).1.to_string(),//get the last part, eg foom.rtd from /home/foom/foom.rtd
        }
    }

    pub fn clear_file_list(&mut self) {}

    ///
    /// This wil read all the '*rtd' file names and return them in result
    ///
    pub fn scan_directory(&mut self, dir_path: &str, extension: &str) -> Result<Vec<String>, Box<dyn Error>> {
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

    ///If list exists, will open it
    pub fn open_list(&mut self, list_name:&str){
        let abs_list_name = if (!list_name.starts_with("./")){
            format!("./{}", list_name)
        }else{
            String::from(list_name)
        };


        debug!("Going to open list if it's found: {}", abs_list_name);
       //can we find the list? open it
       if Path::new(&abs_list_name).exists() {
           if let Ok(()) = db::load_list(self, &abs_list_name){
               self.file_path = String::from(list_name)
           }
       }
    }

    ///Will scan the current directory once, to prevent loop jamming
    pub fn scan_files_once(&mut self) {
        if self.has_scanned {
            return;
        }
        // debug!("Scanning files...");
        //go through the directory
        let tudu_files = match self.scan_directory("./", "rtd") {
            Err(e) => panic!("Unable to open file dialog: {}", e),
            Ok(entries) => entries,
        };
        // debug!("We found {} files!",  &tudu_files.len());
        // tudu_files.i
        tudu_files.iter()
                  .for_each(|s| {
                      // debug!("Pushing {}", s);
                      self.open_file_dialog_files.items.push(String::from(s));
                  });
        self.has_scanned = true;
    }

    ///reset the scan variable
    pub fn reset_scan_guard(&mut self) {
        self.has_scanned = false;
    }
    // fn get_item_tree(&mut self) -> HashMap<u32, Vec<Item>> {
    //     self.item_tree
    // }
}
