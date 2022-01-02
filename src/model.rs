use core::fmt;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Display;
use std::fs::File;
use std::mem;
use std::ops::Index;
use std::os::unix::process::parent_id;
use std::path::Path;

use chrono::{DateTime, Utc};
use log::{debug, error, warn};
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

pub enum MoveDirection {
    ///up sibling list
    Up,
    ///down sibling list
    Down,
    ///become child of item above
    In,
    //become sibling of parent
    Out,
}

impl Display for MoveDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut write_str = match *self {
            MoveDirection::Up => { "Up" }
            MoveDirection::Down => { "Down" }
            MoveDirection::In => { "In" }
            MoveDirection::Out => { "Out" }
        };
        write!(f, "{}", write_str)
    }
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
    ///Order among its siblings
    pub order: u16,
}

impl Item {

    // pub fn new(title:String)->{
    //     Item{title, entry, status: ItemStatus::Undone, parent: Box::new(None)}
    // }

    ///constructor, no parent
    pub fn new(id: u32, title: &str, entry: &str) -> Self {
        // Item { title, entry, parent: Box::new(None) }
        Self {
            id,
            title: title.to_string(),
            entry: entry.to_string(),
            parent_id: 0,
            child_ids: Vec::new(),
            expand: ExpandStatus::Closed,
            complete: CompleteStatus::Incomplete,
            depth: 0,
            order: 0,
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
        let modifier = match self.complete {
            CompleteStatus::Complete => Modifier::CROSSED_OUT | Modifier::ITALIC,
            CompleteStatus::Incomplete => Modifier::empty(),
        };
        let color = match self.complete {
            CompleteStatus::Complete => Color::DarkGray,
            CompleteStatus::Incomplete => Color::White,
        };
        let depth_string = "--".to_string().repeat(self.depth);
        let mut content = vec![Spans::from(
            Span::styled(format!("{}{}.{}: {} {}", depth_string,
                                 &item_no, &self.depth, &self.expansion_state_symbol(), self.title),
                         Style::default().add_modifier(modifier).fg(color)))];
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
            ExpandStatus::Closed => self.expand = ExpandStatus::ShowChildren,
            ExpandStatus::ShowChildren => self.expand = ExpandStatus::Open,
            ExpandStatus::Open => {}//do nothing
        };
    }

    pub fn show_children(&mut self) {
        if self.expand == ExpandStatus::Closed {
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
    pub fn new() -> Self {
        Self {
            state: ListState::default(),
            items: Vec::new(),
        }
    }

    pub fn with_items(items: Vec<T>) -> Self {
        Self {
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
    InsertAtRoot,
    ///inserts at the root level
    InsertChild,
    ///inserts at the same level
    InsertParent,
    InsertSibling,
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
    //how far in from the end of the line are we
    cursor_offset: u16,

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
            cursor_offset: 0,
        }
    }
}

impl RutuduList {
    pub fn enter_edit_mode(&mut self) {
        self.cursor_position = [1, 1];
        self.input_mode = InputMode::Edit;
    }

    pub fn enter_save_mode(&mut self) {
        self.cursor_position = [self.file_path.len() as u16, 1];
        self.cursor_offset = 0;
        self.input_mode = InputMode::Save;
    }

    pub fn mark_saved(&mut self) {
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
    // pub fn enter_child_insert_mode(&mut self) {
    //     debug!("Insert child mode");
    //     // let i = self.items.state.selected().unwrap_or(0);
    //     self.input_mode = InputMode::InsertChild;
    // }
    // pub fn enter_parent_insert_mode(&mut self) {
    //     debug!("Insert parent mode");
    //     // let i = self.items.state.selected().unwrap_or(0);
    //     self.input_mode = InputMode::InsertParent;
    // }
    //
    // pub fn enter_sibling_insert_mode(&mut self) {
    //     debug!("Insert sibling mode");
    //     // let i = self.items.state.selected().unwrap_or(0);
    //     self.input_mode = InputMode::InsertSibling;
    // }

    //Create an item at the current level
    pub fn enter_insert_mode(&mut self, mode: InputMode) {
        self.input_mode = mode;
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
        let num_children = self.item_tree
                               .entry(item_id)
                               .or_insert_with(Vec::new)
                               .len();
        if let Some(bucket) = self.item_tree.get_mut(&parent_id) {//get the list we belong to, could be zero
            bucket.iter_mut().for_each(|item| {
                if item.id == item_id {
                    item.collapse();
                    if num_children == 0 {//roll up all the way
                        item.collapse();
                    }
                }
            })
        };
        self.dirty_list = true;
    }

    ///Move an up and down its siblings
    pub fn move_item(&mut self, dir: MoveDirection) {
        debug!("move_item, direction: {}", dir);
        let i = if let Some(i) = self.items.state.selected() {
            i
        } else {
            //if nothing selected nothing to do
            return;
        };
        //move it in the bucket
        let parent_id = self.items.items[i].parent_id;
        let id = self.items.items[i].id;
        let mut grand_parent_id = 0;
        let mut grand_parent_bucket = 0;
        self.item_tree.iter()
            .for_each(|(k, v)| {
                v.iter().for_each(|i| {
                    if i.id == parent_id {
                        grand_parent_id = i.parent_id;
                    }
                });
            });

        //if we move up or down siblings, we need to find the bucket of siblings and swap
        if let Some(mut parent_child_bucket) = self.item_tree.get_mut(&parent_id) {
            debug!("Found the bucket");
            if let Some(mut idx) = parent_child_bucket
                .iter_mut()
                .position(|item| { item.id == id }) {
                //now we have the idx, we can decide what to do
                match dir {
                    MoveDirection::Up => {
                        let idx_to_swap = if idx == 0 {//first item, loop around
                            parent_child_bucket.len() - 1
                        } else {
                            idx - 1
                        };
                        parent_child_bucket.swap(idx, idx_to_swap);
                        self.items.previous();
                    }
                    MoveDirection::Down => {
                        let idx_to_swap = if idx == parent_child_bucket.len() - 1 {//last time, loop around
                            0
                        } else {
                            idx + 1
                        };
                        // debug!("Going down idx: {} idx_to_swap {}", idx, idx_to_swap);
                        self.items.next();
                        parent_child_bucket.swap(idx, idx_to_swap);
                    }
                    MoveDirection::In => {//become the child of sibling immediately above on list
                        if parent_child_bucket.len() == 1 {//if it's only one on this level, it cannot become its own child
                            return;
                        }

                        //we want to get the id from the sibling immediately above
                        let idx_to_swap = if idx == 0 {
                            parent_child_bucket.len() - 1
                        } else {
                            idx - 1
                        };

                        //get a handle on what will be the new parent - the Sibling Above
                        let sibling_above = parent_child_bucket
                            .get(idx_to_swap)
                            .unwrap()
                            .clone();
                        let new_parent_id = sibling_above.id;

                        //remove from the  original child bucket
                        let mut oi = parent_child_bucket.remove(idx);
                        oi.parent_id = new_parent_id.clone();

                        //put it into the sibling's bucket at the beginning
                        self.item_tree.entry(new_parent_id)
                            .or_insert_with(Vec::new)
                            .insert(0, oi);

                        //expand the new parent if it was not expanded
                        let new_select_index = self.items.items
                                                   .iter()
                                                   .position(|i| i.id == sibling_above.id)
                                                   .unwrap_or(0);

                        if sibling_above.expand == ExpandStatus::Closed {//expand the new parent
                            self.items.state.select(Some(new_select_index));
                            self.expand_selected();
                        }

                        self.items.state.select(Some(new_select_index + 1));//select the new child


                        self.dirty_list = true;
                    }
                    MoveDirection::Out => {//make your grandparent your parent id and put yourself in the right bucket
                        if parent_id == 0 {//if it's at the root level, we are as far out (man) as we can go
                            return;
                        }
                        //how to find one's parent....
                        let parent = self.items.items.iter()
                                         .find(|i| i.id == parent_id)
                                         .unwrap();

                        //take out of the old vector
                        let mut oi = parent_child_bucket.remove(idx);
                        oi.parent_id = grand_parent_id;

                        //what is the  new sibling's id, we want to go after it
                        let parent_idx = self.item_tree.get(&grand_parent_id)
                                             .unwrap()
                                             .iter()
                                             .position(|i| i.id == parent_id)
                                             .unwrap_or(0);


                        let mut offset = if self.item_tree.get(&grand_parent_id)
                                                .unwrap()
                                                .len() > 0 {
                            1
                        } else {
                            0
                        };

                        let fin_idx = parent_idx.clone() + offset;
                        self.item_tree.entry(grand_parent_id)
                            .or_insert_with(Vec::new)
                            .insert(fin_idx, oi);


                        self.dirty_list = true;
                    }
                }
            } else {
                debug!("Did not find any bucket? {} ", parent_id );
                error!("Unable to navigate through bucket to move items");
            }
        } else {
            debug!("Did not find any bucket? {} ", parent_id );
        }
        self.dirty_list = true;
    }

    ///Moves expansion status up the scale
    pub fn expand_selected(&mut self) {
        let i = self.items.state.selected().unwrap_or(0);
        let list_size = self.items.items.len();
        debug!("Expanding item with selected index {} on list of size {}", i, list_size);
        //expand the parent
        // self.items.items[i].expand(); // this doesn't matter because we are rebuilding the list from the map
        //get the parent id and then get the item and set its expansion status
        let parent_id = self.items.items[i].parent_id;
        let item_id = self.items.items[i].id;
        let num_children = self.item_tree
                               .entry(item_id)
                               .or_insert_with(Vec::new)
                               .len();

        debug!("parent_id={}, item_id={}, num_children={}", parent_id, item_id, num_children);

        self.item_tree
            .entry(parent_id)
            .or_insert_with(Vec::new)
            .iter_mut()
            .filter(|i| i.id == item_id)
            .for_each(|i| {
                debug!("We are going to expand");
                i.expand();
                if num_children == 0 {//no children, expand all the way
                    i.expand();
                }
            });
        self.dirty_list = true;
    }

    pub fn delete_selected(&mut self) {
        let i = if let Some(index) = self.items.state.selected() {
            index
        } else {
            return;//nothing selected, nothing to delete
        };

        //find item to delete
        let item_id = self.items.items[i].id;
        //find its parent
        let grand_parent_id = self.items.items[i].parent_id;

        //find its children and set the parent ids of the children to the parents parent
        self.item_tree
            .entry(i as u32)
            .or_insert_with(Vec::new)
            .iter_mut()
            .for_each(|c| c.parent_id = grand_parent_id);

        //now we want to remove the child vector from the item tree, and add it to the grandparent id
        //remove the parent from the representative tree, and stick the child bucket in
        // let grand_parent_bucket = self.item_tree.entry(grand_parent_id)
        //                               .or_insert_with(Vec::new);
        //remove selected item
        if let Some(item_idx) = self.item_tree
            .entry(grand_parent_id)
            .or_insert_with(Vec::new)
            .iter()
            .position(|c| c.id == item_id){
                self.item_tree
                    .entry(grand_parent_id)
                    .or_insert_with(Vec::new)
                    .remove(item_idx);
        }

        //could I have used the "move" functionality for this? hmmmm dunno
        //get all the child items of removed parent
        let mut item_bucket: Vec<Item> = self.item_tree
            .get_mut(&item_id)
            .unwrap_or(&mut Vec::new())
            .drain(..)
            .collect();
        //need to reset the parent id
        item_bucket.iter_mut()
            .for_each(|c| c.parent_id = grand_parent_id);
        self.item_tree.entry(grand_parent_id).or_insert_with(Vec::new).append(&mut item_bucket);

        //now we need to change the selection to one higher....

        self.items.previous();
        //rebuild the list
        self.dirty_list = true;
    }

    fn move_children_to_new_bucket(&mut self, new_parent_id:u32, old_bucket: &mut Vec<Item>){
       self.item_tree.entry(new_parent_id).or_insert_with(Vec::new).append(old_bucket);
    }

    pub fn toggle_selected_item_completion_status(&mut self) {
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
            self.unsaved = true;
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
        root_items_vec.iter().enumerate().for_each(|(i, item)| {
            // let new_item = ListItem::new(item.text(i));
            self.items.items.push(item.clone());
        });
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
            }
        };
        item_text_as_vec
    }

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
        self.insert_item(&mut item);

        //and we also keep a list of all the items

        self.enter_edit_mode();
    }

    ///Add item as parent to currently selected item, or, if noe, just add it


    pub fn insert_item(&mut self, item: &mut Item) {
        debug!("Adding item, id: {} ", item.id);
        if item.parent_id == 0 {//do we need to set a parent?
            item.parent_id = match self.input_mode {
                InputMode::InsertChild => {//parent is the node we selected
                    if let Some(i) = self.items.state.selected() {
                        //children lists are by implication - mapped by item.id in the hashmap
                        self.items.items[i].id.clone()
                    } else { 0 }
                }
                InputMode::InsertSibling => {//parent is parent of node we selected
                    if let Some(i) = self.items.state.selected() {
                        self.items.items[i].parent_id.clone()
                    } else { 0 }
                }
                InputMode::InsertParent => {//
                    //we make it it's own parent, after insertion we're going to swap it
                    //with the selected item and swap their parent ids
                    item.id
                }
                _ => { 0 }
            }
        }
        //this will leave parent ids intact, which is good for when we open up lists
        debug!("Parent id: {}", item.parent_id.clone());

        //now we place the item in the right bucket
        let mut new_item = item.clone();
        let bucket = self.item_tree
                         .entry(item.parent_id.clone())
                         .or_insert_with(Vec::new);
        new_item.order = bucket.len() as u16;
        bucket.push(new_item);

        //now we do postprocessing
        if self.input_mode == InputMode::InsertParent {
            self.add_item_as_parent(item)
        } else if self.input_mode == InputMode::InsertChild {
            let opt_parent = match self.items.state.selected() {
                Some(i) => self.items.items.get_mut(i),
                None => None,
            };
            if let Some(list_item) = opt_parent {
                self.item_tree.entry(list_item.parent_id.clone())
                    .or_insert_with(Vec::new)
                    .iter_mut()
                    .filter(|i| { i.id == list_item.id })
                    .for_each(|i| { i.show_children() })
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
        // content - we set the id to the maximum id +i
        let mut max_id = 0;
        self.item_tree.iter()
            .for_each(|(x,v)| {
               v.iter().for_each(|c| {
                   if c.id > max_id{
                       max_id = c.id;
                   }
               })
            });
                                // .max()
        debug!("Next item id: {}", max_id+1);
        // let i = self.items.items.len().clone() as u32;
        //we want to start this at ONE so we reserve the zero index for the root nodes of the forest
        Item::new((max_id as u32) + 1, &title, &entry)
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

    pub fn cursor_left(&mut self) {
        if self.cursor_position[0] > 0 {
            self.cursor_position[0] = self.cursor_position[0] - 1;
        }
    }

    pub fn cursor_right(&mut self) {
        if self.cursor_position[0] < self.current_item.len() as u16 {
            self.cursor_position[0] = self.cursor_position[0] + 1;
        }
    }

    ///TODO remove a character where the cursor is and implement delete
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

    pub fn left_save_cursor(&mut self) {
        debug!("Move save cursor left, cursor[0] = {} ", self.cursor_position[0]);
        if self.cursor_position[0] > 0 {
            self.cursor_position[0] = self.cursor_position[0] - 1;
            self.cursor_offset += 1;
        } else {
            debug!("Not moving?");
        }
    }

    pub fn right_save_cursor(&mut self) {
        debug!("Move save cursor right, cursor[0] = {} ", self.cursor_position[0]);
        if self.cursor_position[0] < self.file_path.len() as u16 {
            self.cursor_position[0] = self.cursor_position[0] + 1;
            self.cursor_offset -= 1;
        }
    }

    pub fn add_save_input_char(&mut self, c: char) {
        //we insert at the cursor position
        let insert_index = self.file_path.len() as u16 - self.cursor_offset;
        self.file_path.insert(insert_index as usize, c);
        // self.file_path.push(c);
        self.cursor_position[0] = self.cursor_position[0] + 1;
    }

    pub fn remove_save_file_char(&mut self) {
        if self.file_path.is_empty() {
            return;
        }
        debug!("remove_save_file_char, where x={}, y={} ", self.cursor_position[0], self.cursor_position[1]);

        // self.file_path.pop();

        //-1 because we want to delete BEHIND the cursor
        let delete_pos = self.file_path.len() - 1 - self.cursor_offset as usize;
        self.file_path.remove(delete_pos);
        self.cursor_position[0] -= 1;
        // self.cursor_position[0] = self.file_path.len() as u16;
        // debug!("x={}, y={} ", self.cursor_position[0], self.cursor_position[1]);
    }

    pub fn list_name(&mut self) -> String {
        //we add an asterisk if it is unsaved
        let save_needed = if self.unsaved { "*" } else { "" };
        let fp = format!("{}{}", self.file_path.clone(), save_needed);
        //trim off the first path of the filepath`
        match fp.rfind('/') {
            None => fp,
            Some(i) => fp.split_at(i + 1).1.to_string(),//get the last part, eg foom.rtd from /home/foom/foom.rtd
        }
    }

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
    pub fn open_list(&mut self, list_name: &str) {
        let abs_list_name = if (!list_name.starts_with("./")) {
            format!("./{}", list_name)
        } else {
            String::from(list_name)
        };


        debug!("Going to open list if it's found: {}", abs_list_name);
        //can we find the list? open it
        if Path::new(&abs_list_name).exists() {
            if let Ok(()) = db::load_list(self, &abs_list_name) {
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

#[cfg(test)]
mod tests{
    use super::*;

    //deleting is doing super weird things
    //why not try and test it? like a real little programmer
    // #[test]
    // fn test_delete_then_add


}
