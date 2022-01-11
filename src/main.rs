use std::error::Error;
use std::io::Stdout;

use chrono::prelude::*;
use clap::{App, ArgMatches, Arg};
use log::{debug, error, LevelFilter};
use regex::Regex;
use log4rs;
use termion::{clear, raw::IntoRawMode};
use termion::event::Key;
use termion::raw::RawTerminal;
use tui::{Frame, Terminal};
use tui::backend::TermionBackend;
use tui::layout::{Constraint, Direction, Layout, Rect, Alignment};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, BorderType};

use model::InputMode;

use crate::events::{Event, Events};
use crate::model::{MoveDirection, RutuduList, StatefulList};
use rusqlite::Connection;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::{Config, Handle};
use log4rs::config::{Appender, Root};
use std::path::Path;

mod events;
mod model;
mod db;
use num_traits::cast::ToPrimitive;

// const DATE_FMT: &str = "%Y%m%d%H%M%s";
const DATE_FMT: &str = "%Y%m%d";

fn init() -> ArgMatches {
    App::new("Rutudu Todo List")
        .version("1.0")
        .author("FOOM")
        .about("Todo List, Terminal style, Rust vibes")
        .arg(Arg::new("list_name")
            .value_name("list_name")
            .about("Name of list, will default to 'rutudu$DATE.rtd' if not supplied. Name of sqlite file.")
            .index(1)
        .required(false))
        .arg("-v, --verbose 'All that info'")
        .arg("-d, --create-default 'Create rutudu$DATE.db'")
        .get_matches()
}

fn init_logger(verbose: bool) {
    let log_level = if verbose {
        LevelFilter::Debug
    } else {
        LevelFilter::Warn
    };
    let stdout = ConsoleAppender::builder().build();
    let file_logger = FileAppender::builder().build("./rutudu.log").unwrap();

    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("file_logger", Box::new(file_logger)))
        .build(Root::builder()
            // .appender("stdout")
            .appender("file_logger")
            .build(log_level))
        .unwrap();

    log4rs::init_config(config).unwrap();
    debug!("Debugging has been initialized");
}


///Create "rutuduTODAY.db" file with rutudu_list table where today is %Y%m%d
// fn create_default_rutudu_list() -> Result<String, Box<dyn Error>> {
//     //get the date
//     let today = Utc::now().format(DATE_FMT);
//     debug!("About to create rutudu list rutudu$DATE.db");
//     //create sqlite connection to rutudu$DATE.db
//     let list_name = format!("./rutudu{}.db", today);
//     let connection = Connection::open(&list_name).unwrap();
//     connection.execute("
//         CREATE TABLE rutudu_list(
//             id INTEGER PRIMARY KEY ASC,
//             parent_id INTEGER,
//             title TEXT NOT NULL,
//             entry TEXT
//         );
//     ", [])?;
//     //return the rutudu name
//     return Ok(list_name);
// }

/// helper function to create a centered rect using up
/// certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Min(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ].as_ref(),
        ).split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ].as_ref(),
        )
        .split(popup_layout[1])[1]
}

///For little popups we want to display in the middle of the screen, eg quit, etc
fn little_popup(min_horizontal:u16, min_vertical:u16, r:Rect) -> Rect {
    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage(30),
                Constraint::Min(min_vertical),
                Constraint::Percentage(70),
            ].as_ref(),
        ).split(r);

    //now we split the middle vertical one into three and return the middle one of that!
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(40),
                Constraint::Min(min_horizontal),
                Constraint::Percentage(60),
            ].as_ref(),
        ).split(v[1])[1]

}

// fun make_vec_for_lest(items: )
fn get_default_list_name()->String{
    debug!("No name arg passed...");
    let today = Utc::now().format(DATE_FMT);
    format!("./rutudu{}.rtd", today)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = init();
    init_logger(args.is_present("verbose"));
    let default_name = get_default_list_name();
    // let list_name = args.value_of("list_name").unwrap_or(&default_name);
    let list_name = args.value_of("list_name").unwrap_or(&default_name);
    //setup the gui display
    let stdout = match std::io::stdout().into_raw_mode() {
        Ok(outstream) => outstream,
        Err(e) => panic!("Couldn't open up the outstream: {}", e),
    };
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let events = Events::new();
    // println!("{}", clear::All);
    //display first/appropriate todolist
    //display all cell and it's children (recursively)
    //display add new list
        //we want the screen to be cleared
    println!("{}", clear::All);
    //like in vim, 2 modes, edit and insert
        //few more modes now, we in state machine territory
    // let mut edit_mode = true;
    let mut tudu_list = RutuduList::default();
    tudu_list.file_path = list_name.to_string();
    tudu_list.open_list(list_name);

    // let mut items = [ListItem::new("Item 1"),
    //     ListItem::new("Item 2"), ListItem::new("Item 3")];
    loop {
        terminal.draw(|f| {
            //get the map and then build a new list and display it
            let title = tudu_list.list_name();
            // let mut items: Vec<ListItem> = tudu_list.items_as_vec();
            // tudu_list.clear_list();
            tudu_list.rebuild_list_if_dirty();
            let item_list = tudu_list.items_as_vec();
            let items = item_list.clone();
            let mut lst_state = tudu_list.items.state.clone();
            let tui_items = List::new(items)
                .block(Block::default().title(title).borders(Borders::ALL))
                // .style(Style::default().fg(Color::White))
                .highlight_style(Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Cyan))
                .highlight_symbol(">");


            let size = f.size();
            //split into 3
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(2),
                    Constraint::Length(3),
                ].as_ref() )
                .split(size);

            let mnemonics_text = ["Add", "X-out", "Save", "Open", "Quit", ];
            let mnemonics:Vec<Span> = mnemonics_text
                .iter()
                .cloned()
                .flat_map(|t| {
                    let (first, rest) = t.split_at(1);
                        vec![
                            Span::styled(" [", Style::default().fg(Color::Yellow)),
                            Span::styled( first, Style::default().fg(Color::Yellow).add_modifier(Modifier::UNDERLINED)),
                            Span::styled("]", Style::default().fg(Color::Yellow)),
                            Span::styled(rest, Style::default().fg(Color::LightYellow)),
                        ]
                }).collect();

            let menu = Spans::from(mnemonics);
            let top_text = Paragraph::new(Spans::from(Span::styled("R U T U D U",
                                                                   Style::default()
                                                                       .fg(Color::LightCyan)
                                                                       .add_modifier(Modifier::BOLD))));

            f.render_widget(top_text, chunks[0]);
            let bottom_text = Paragraph::new(menu)
                .style(Style::default().fg(Color::Cyan))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .style(Style::default().fg(Color::LightGreen))
                        .title("[M]nemonics")
                        .border_type(BorderType::Double), );

            f.render_stateful_widget(tui_items, chunks[1], &mut lst_state);

            f.render_widget(bottom_text, chunks[2]);

            match tudu_list.input_mode {
                InputMode::InsertAtRoot | InputMode::InsertChild| InputMode::InsertParent | InputMode::InsertSibling =>  show_new_item_input(&mut tudu_list, f),
                InputMode::Quit => draw_quit_dialog(f),
                InputMode::Save => draw_save_dialog(&mut tudu_list,f),
                InputMode::Open =>  draw_open_dialog(&mut tudu_list,f),
                InputMode::Edit =>  {},
            }
        }).unwrap();

        if let Event::Input(input) = events.next()? {
            match tudu_list.input_mode {
                InputMode::Edit => match input {
                    Key::Char('q') => tudu_list.enter_quit_mode(),
                    Key::Char('s') => tudu_list.enter_save_mode(),
                    Key::Char('o') => tudu_list.enter_open_mode(),
                    Key::Char('x') => tudu_list.toggle_selected_item_completion_status(),//println!("{}", clear::All);
                        // break;

                    Key::Char('d') => tudu_list.move_item(MoveDirection::Down),
                    Key::Char('u') => tudu_list.move_item(MoveDirection::Up),
                    Key::Char('>') => tudu_list.move_item(MoveDirection::In),
                    Key::Char('<') => tudu_list.move_item(MoveDirection::Out),

                    Key::Char('h') | Key::Left => tudu_list.collapse_selected(),
                    Key::Char('j') | Key::Down => tudu_list.down(),
                    Key::Char('k') | Key::Up =>  tudu_list.up(),
                    Key::Char('l') | Key::Right =>  tudu_list.expand_selected(),

                    Key::Delete | Key::Backspace => tudu_list.delete_selected(),

                    Key::Char('a') => tudu_list.enter_insert_mode(InputMode::InsertAtRoot),
                    Key::Ctrl('a') => tudu_list.enter_insert_mode(InputMode::InsertChild),
                    Key::Alt('a') => tudu_list.enter_insert_mode(InputMode::InsertParent),
                    Key::Char('A') => tudu_list.enter_insert_mode(InputMode::InsertSibling),

                    _ => {}
                },
                InputMode::InsertAtRoot | InputMode::InsertChild | InputMode::InsertParent | InputMode::InsertSibling => match input {

                    //what's a better key combo? ctrl+[ does weird things...
                    //terminal doesn't support ctrl+\n, ctrl/shift don't modify the key being pressed dammit
                    //alt+\n just does not seem to work?
                    Key::Alt(c) => if c as u32 == 13 {
                        debug!("Alt was pressed with enter!!");
                        tudu_list.add_input_text_as_item_to_list();
                    }
                    else{
                        debug!("We pressed alt+{}", c);
                        debug!("Ascii val == {}", c as u32);
                    }
                    Key::Ctrl('n') =>  tudu_list.add_input_text_as_item_to_list(),
                    Key::Backspace => tudu_list.remove_character(),
                    Key::Left => tudu_list.cursor_left(),
                    Key::Right => tudu_list.cursor_right(),
                    Key::Char(c) => tudu_list.add_character(c),//tudu_list.current_item.push(c),
                    Key::Esc => tudu_list.enter_edit_mode(),
                    // Key::Char(c) => {println!("{}", c)}
                    Key::Ctrl(c) => { println!("{}", c) }
                    _ => {}
                },
                InputMode::Save => match input{
                    Key::Ctrl('n') => {
                        db::save_list(&tudu_list).unwrap();
                        tudu_list.mark_saved();
                    }
                    Key::Char(c) => if '\n' == c {
                        db::save_list(&tudu_list).unwrap();
                        tudu_list.mark_saved();
                    }else{
                        tudu_list.add_save_input_char(c);
                    },
                    Key::Ctrl('\n') => {//how can I combine with the above?
                        db::save_list(&tudu_list).unwrap();
                        tudu_list.mark_saved();

                    }
                    Key::Left => tudu_list.left_save_cursor(),
                    Key::Right => tudu_list.right_save_cursor(),
                    Key::Backspace => tudu_list.remove_save_file_char(),
                    Key::Esc => tudu_list.enter_edit_mode(),
                    _ => {}
                },
                InputMode::Open => match input{//allow moving up and down to select
                    Key::Char('j') | Key::Down => tudu_list.open_file_down(),
                    Key::Char('k') | Key::Up => tudu_list.open_file_up(),
                    Key::Char('l') | Key::Right | Key::Char('\n') | Key::Ctrl('n') =>tudu_list.load_list_from_file_dialog(),
                    Key::Esc => tudu_list.enter_edit_mode(),
                    _ => {}
                },
                InputMode::Quit => match input {
                    Key::Char('y') | Key::Char('\n') => {
                        println!("{}", clear::All);
                        break;
                    },
                    Key::Char('n') | Key::Esc => tudu_list.enter_edit_mode(),
                    _ => {}
                }
            }
        };
    };

    Ok(())
}

fn show_new_item_input(tudu_list: &mut RutuduList, f: &mut Frame<TermionBackend<RawTerminal<Stdout>>>) {
    let size = f.size();
    let input_box = Paragraph::new(tudu_list.current_item.as_ref())
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().title("Todo Item").borders(Borders::ALL));
    // let input_box_rect = Rect::new(rect.x + 20, rect.y + 20, 150, 16);
    let area = centered_rect(60, 20, size);
    f.render_widget(Clear, area); //this clears out the background
    f.render_widget(input_box, area);

    f.set_cursor(area.x as u16 + tudu_list.cursor_position[0], area.y as u16 + tudu_list.cursor_position[1]);
}

fn draw_quit_dialog(f: &mut Frame<TermionBackend<RawTerminal<Stdout>>>) {
    let rect = f.size();
    // let quit_text = Paragraph::new("Really quit?")
    //     .style(Style::default().fg(Color::Cyan))
    //     .block(Block::default().borders(Borders::ALL));
    let button_text = Paragraph::new("[Y][N]")
        .style(Style::default().fg(Color::Cyan))
        .block(Block::default().borders(Borders::ALL).title("Really Quit?"));
    // let area = centered_rect(10, 16, size);
    let area = little_popup(20, 3, rect);

    f.render_widget(Clear, area);
    // f.render_widget(quit_text, quit_chunks[0]);
    f.render_widget(button_text, area);
}

fn draw_save_dialog(tudu_list:&mut RutuduList, f: &mut Frame<TermionBackend<RawTerminal<Stdout>>>){
    let rect = f.size();
    let save_text = Paragraph::new(tudu_list.file_path.clone())
        .style(Style::default().fg(Color::Cyan))
        .block(Block::default().borders(Borders::ALL).title("[S]ave?"));
    let area = little_popup(40,5, rect);

    f.render_widget(Clear,area);
    f.render_widget(save_text, area);
    // tudu_list.cursor_position[0] =
    // f.set_cursor(area.x as u16 + tudu_list.file_path.len() as u16 +1, area.y as u16 + tudu_list.cursor_position[1] );
    f.set_cursor(area.x as u16 + tudu_list.cursor_position[0] as u16 +1, area.y as u16 + tudu_list.cursor_position[1] );
}

fn draw_open_dialog(tudu_list: &mut RutuduList, f: &mut Frame<TermionBackend<RawTerminal<Stdout>>>) {
    tudu_list.scan_files_once();

    // debug!("Trying to draw open dialog");
    // let lst_tudu_files = StatefulList::with_items(tudu_files);
    //get the files
    // tudu_list.scan_files_once();
    let tudu_files = tudu_list.open_file_dialog_files.items.clone();
    // debug!("We have these many files: {}", tudu_files.len());

    let mut tudu_file_state = tudu_list.open_file_dialog_files.state.clone();
    let uheight = tudu_files.len();
    let uh = uheight.to_u16().unwrap_or(10)*10;
    let rect = little_popup(40,uh,f.size());

    let tudu_spans:Vec<ListItem> = tudu_files.iter()
                                             .map(|f|{
                                                 let file_name = Spans::from(Span::raw(f));
                                                 ListItem::new(file_name)
                                             }).collect();
    let file_items = List::new(tudu_spans)
        .block(Block::default()
            .title("Open list...")
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .style(Style::default().fg(Color::Cyan)))
        .style(Style::default()
            .fg(Color::LightCyan))
        .highlight_style(Style::default()
            .add_modifier(Modifier::BOLD).fg(Color::LightBlue))
        .highlight_symbol("o");

    f.render_widget(Clear,rect);
    f.render_stateful_widget(file_items, rect,&mut tudu_file_state);

}
