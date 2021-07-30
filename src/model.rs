use tui::widgets::ListState;

pub enum ItemStatus{
    Undone,
    Done,
}
///Represent items on the todo list
pub struct Item {
    pub title: String,
    pub entry: String,
    pub status: ItemStatus,
    ///if None, this is on the root level
    pub parent: Box<Option<Item>>,
    pub expand: bool,
}

impl Item {

    // pub fn new(title:String)->{
    //     Item{title, entry, status: ItemStatus::Undone, parent: Box::new(None)}
    // }

    ///root constructor, no parent
    pub fn new(title: String, entry: String) -> Item {
        // Item { title, entry, parent: Box::new(None) }
        Item{title, entry, status: ItemStatus::Undone, parent: Box::new(None), expand:false}
    }
    ///yep, has a parent but may not be leaf
    pub fn new_child(title: String, entry: String, parent: Item) -> Item {
        Item { title, entry, status: ItemStatus::Undone, parent: Box::new(Some(parent)), expand:false }
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

pub enum InputMode {
    Insert,
    Edit,
}

pub struct RutuduList {
    ///what mode are we in?
    pub input_mode: InputMode,
    ///start with strings, advance to items
    // pub items:Vec<String>,

    pub items: StatefulList<Item>,

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
        }
    }
}

impl RutuduList {
    pub fn enter_edit_mode(&mut self) {
        self.input_mode = InputMode::Edit;
    }
    pub fn enter_insert_mode(&mut self) {
        self.input_mode = InputMode::Insert;
    }

    pub fn close_selected(&mut self){
        let i = self.items.state.selected().unwrap_or(0);
        self.items.items[i].expand = true;
    }

    pub fn open_selected(&mut self){
        let i = self.items.state.selected().unwrap_or(0);
        self.items.items[i].expand = false;
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
        Item{title, entry, expand: false, parent: Box::new(None), status: ItemStatus::Undone}
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

}