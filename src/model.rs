use tui::widgets::ListState;

///Represent items on the todo list
pub struct Item {
    pub title: String,
    pub entry: String,
    ///if None, this is on the root level
    pub parent: Box<Option<Item>>,
}

impl Item {
    ///root constructor, no parent
    pub fn new(title: String, entry: String) -> Item {
        Item { title, entry, parent: Box::new(None) }
    }
    ///yep, has a parent but may not be leaf
    pub fn new_child(title: String, entry: String, parent: Item) -> Item {
        Item { title, entry, parent: Box::new(Some(parent)) }
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
    ///current value of the input box
    pub input: String,
    ///what mode are we in?
    pub input_mode: InputMode,
    ///start with strings, advance to items
    // pub items:Vec<String>,

    pub items: StatefulList<String>,

    pub current_item: String,
    //todo items
    // items:Vec<Item>
}

///New todolist out of nuffink
impl Default for RutuduList {
    fn default() -> Self {
        RutuduList {
            input: String::new(),
            input_mode: InputMode::Edit,
            items: StatefulList::new(),
            current_item: "".to_string(),
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
    pub fn down(&mut self){
       self.items.next();
    }

    pub fn up(&mut self){
        self.items.previous();
    }
}