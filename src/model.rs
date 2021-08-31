use tui::text::{Span, Spans};
use tui::widgets::{ListItem, ListState};
use tui::style::{Style, Modifier};
use std::fs::File;
use std::path::Path;
use std::error::Error;

pub enum ItemStatus{
    Undone,
    Done,
}
///Represent items on the todo list
pub struct Item {
    pub title: String,
    pub entry: String,
    ///if None, this is on the root level
    pub parent: Box<Option<Item>>,
    pub expand: bool,
    pub complete: bool,
}

impl Item {

    // pub fn new(title:String)->{
    //     Item{title, entry, status: ItemStatus::Undone, parent: Box::new(None)}
    // }

    ///root constructor, no parent
    pub fn new(title: String, entry: String) -> Item {
        // Item { title, entry, parent: Box::new(None) }
        Item{title, entry, parent: Box::new(None), expand:false, complete:false, }
    }
    ///yep, has a parent but may not be leaf
    pub fn new_child(title: String, entry: String, parent: Item) -> Item {
        Item { title, entry, parent: Box::new(Some(parent)), expand:false, complete:false, }
    }

    ///Symbol to indicate if item is expanded or collapsed
    pub fn expansion_state_symbol(&self)->String{
        if self.expand{
            String::from("[-]")
        }else{
            String::from("[+]")
        }
    }

    ///Return the item as text, either just the title,
    /// or the title and the entry, depending on expand status
    pub fn text(&self, item_no:usize) -> Vec<Spans> {
        let mut modifier = Modifier::empty();
        if self.complete {
            modifier = Modifier::CROSSED_OUT;
        }
        let mut content = vec![Spans::from(
            Span::styled(format!("{}: {} {}",
                              &item_no, &self.expansion_state_symbol(), self.title),
            Style::default().add_modifier(modifier)))];
        //show our expanded content if need be
        if self.expand {
           content.push(Spans::from(Span::raw(format!("    {}", self.entry))));
        }
        content
    }
}

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
    Edit,
    Save,
}

pub struct RutuduList {
    ///what mode are we in?
    pub input_mode: InputMode,
    pub items: StatefulList<Item>,

    ///if the list has been saved, this is where
    pub file_path:String,

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
            current_item: "".to_string(),
            cursor_position: [ 1,1 ],
            file_path: String::new()
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
    pub fn enter_insert_mode(&mut self) {
        self.input_mode = InputMode::Insert;
    }

    pub fn close_selected(&mut self){
        let i = self.items.state.selected().unwrap_or(0);
        self.items.items[i].expand = false;
    }

    pub fn open_selected(&mut self){
        let i = self.items.state.selected().unwrap_or(0);
        self.items.items[i].expand = true;
    }

    pub fn toggle_selected_status(&mut self){
        let i = self.items.state.selected().unwrap_or(0);
        self.items.items[i].complete = !self.items.items[i].complete;
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
            self.items.items[i].expand = true;
        }
    }

    pub fn left(&mut self){
        if let Some(i) = self.items.state.selected(){
            self.items.items[i].expand = false;
        }
    }

    pub fn get_current_input_as_item(&mut self) ->Item{
        let mut entry:String = self.current_item.drain(..).collect();
        //split by newlines
        let first_new_line = entry.find("\n").unwrap_or(entry.len());
        let title = entry.drain(..first_new_line).collect();
        // content
        Item{title, entry, expand: false, parent: Box::new(None), complete: false}
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
}