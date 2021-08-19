use std::error::Error;

use chrono::prelude::*;
use clap::{App, ArgMatches};
use log::{debug, error, LevelFilter};
use regex::Regex;
use simple_logger::SimpleLogger;
use termion::{clear, raw::IntoRawMode};
use termion::event::Key;
use tui::backend::TermionBackend;
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::{Terminal, Frame};
use tui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};

use model::InputMode;

use crate::events::{Event, Events};
use crate::model::{Item, RutuduList};
use tui::text::{Span, Spans};
use unicode_width::UnicodeWidthStr;
use termion::raw::RawTerminal;
use std::io::Stdout;

mod events;
mod model;

const DATE_FMT: &str = "%Y%m%d";

fn init() -> ArgMatches {
    App::new("Rutudu Todo List")
        .version("1.0")
        .author("FOOM")
        .about("Todo List, Terminal style, Rust vibes")
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
    SimpleLogger::new()
        .with_level(log_level)
        .init()
        .unwrap();
}

///
/// This wil read all the 'rutudu*db' file names and return them in result
///
fn scan_directory() -> Result<Vec<String>, Box<dyn Error>> {
    let mut lists = Vec::new();
    let rx = Regex::new(r"rutudu.*\.db")?;
    //get the current directory,
    let current_dir = std::env::current_dir()?;
    for entry in std::fs::read_dir(current_dir)? {
        let entry = entry?;
        let path = entry.path();
        let path_str = path.into_os_string().into_string().unwrap();
        if rx.is_match(&path_str) {
            &lists.push(path_str);
        }
    }
    Ok(lists)
}

///Create "rutuduTODAY.db" file with rutudu_list table where today is %Y%m%d
fn create_default_rutudu_list() -> Result<String, Box<dyn Error>> {
    //get the date
    let today = Utc::today().format(DATE_FMT);
    debug!("About to create rutudu list rutudu$DATE.db");
    //create sqlite connection to rutudu$DATE.db
    let list_name = format!("./rutudu{}.db", today);
    let connection = sqlite::open(&list_name).unwrap();
    connection.execute("
        CREATE TABLE rutudu_list(
            id INTEGER PRIMARY KEY ASC,
            parent_id INTEGER,
            title TEXT NOT NULL,
            entry TEXT
        );
    ")?;
    //return the rutudu name
    return Ok(list_name);
}

/// helper function to create a centered rect using up
/// certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
                .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
                .as_ref(),
        )
        .split(popup_layout[1])[1]
}

// fun make_vec_for_lest(items: )

fn main() -> Result<(), Box<dyn Error>> {
    let args = init();
    init_logger(args.is_present("verbose"));
    //#######
    //LET'S FIRST GET SOME SIMPLE STRINGS WORKING
    //get todolists
    // let mut lists = match scan_directory() {
    //     Ok(paths) => paths,
    //     Err(e) => panic!("Unable to scan current directory for paths because {}", e),
    // };
    // if lists.is_empty() {
    //     debug!("No existing rutudu lists found");
    //     if args.is_present("create-default") {
    //         match create_default_rutudu_list() {
    //             Ok(name) => lists.push(name),
    //             Err(e) => error!("Failed to create default list{}", e),
    //         }
    //     }
    // }
    // for path in lists {
    //     debug!("This is the path {}", path);
    // }
    //#######
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

    //like in vim, 2 modes, edit and insert
    // let mut edit_mode = true;
    let mut tudu_list = RutuduList::default();
    // let mut items = [ListItem::new("Item 1"),
    //     ListItem::new("Item 2"), ListItem::new("Item 3")];
    let mut show_quit_dialog = false;
    loop {
        terminal.draw(|f| {

            let mut lst_state = tudu_list.items.state.clone();
            let mut items:Vec<ListItem> = tudu_list.items_as_vec();
            let tui_items = List::new(items)
                .block(Block::default().title("Rutudu").borders(Borders::ALL))
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))
                .highlight_symbol(">");


            let size = f.size();
            let rect = centered_rect(40, 100, size);

            f.render_stateful_widget(tui_items, rect, &mut lst_state);

            match tudu_list.input_mode {
                InputMode::Insert => {
                    let input_box = Paragraph::new(tudu_list.current_item.as_ref())
                        .style(Style::default().fg(Color::Yellow))
                        .block(Block::default().title("Todo Item").borders(Borders::ALL));
                    // let input_box_rect = Rect::new(rect.x + 20, rect.y + 20, 150, 16);
                    let area = centered_rect(60, 20, size);
                    f.render_widget(Clear, area); //this clears out the background
                    f.render_widget(input_box, area);

                    f.set_cursor(area.x as u16 + tudu_list.cursor_position[0], area.y  as u16+ tudu_list.cursor_position[1]);
                    // f.set_cursor(&area.x +tudu_list.current_item.width()  as u16+ tudu_list.cursor_position[0], area.y  as u16+ tudu_list.cursor_position[1])


                }
                InputMode::Edit => {
                    if show_quit_dialog{
                        draw_quit_dialog(f, size);
                    };
                }
            }
        });

        if let Event::Input(input) = events.next()? {
            match tudu_list.input_mode {
                InputMode::Edit => match input {
                    Key::Char('q') => {
                        show_quit_dialog = true;
                    }
                    Key::Char('x') => {
                        tudu_list.toggle_selected_status();//println!("{}", clear::All);
                        // break;
                    }
                    Key::Char('h') | Key::Left => {
                        tudu_list.close_selected();
                    }
                    Key::Char('j') | Key::Down => {
                        tudu_list.down();
                    }
                    Key::Char('k') | Key::Up => {
                        tudu_list.up();
                    }
                    Key::Char('l') | Key::Right => {
                        tudu_list.open_selected();
                        // println!("{}", clear::All);
                        // break;
                    }
                    Key::Char('a') => {
                        tudu_list.enter_insert_mode();
                    }
                    Key::Char('y') => if show_quit_dialog {
                        println!("{}", clear::All);
                        break;
                    }

                    Key::Char('n') => if show_quit_dialog {
                        show_quit_dialog = false;
                    }
                    _ => {}
                },
                InputMode::Insert => match input {
                    //what's a better key combo? ctrl+[ does weird things...
                    //terminal doesn't support ctrl+\n
                    Key::Ctrl('n') => {
                    //     Key::Ctrl('[') => {//this did not work, does escape, like vim
                        //TODO split this into title and entry
                        // let title = tudu_list.current_item.drain(..).collect();
                        // //this should be everything from 2nd line onward
                        // let entry =  String::new();
                        // let item = Item::new(title, entry);
                        let item = tudu_list.get_current_input_as_item();
                        tudu_list.items.items.push(item);
                        tudu_list.enter_edit_mode();
                    },
                    Key::Backspace => tudu_list.remove_character(),
                    Key::Char(c) => tudu_list.add_character(c),//tudu_list.current_item.push(c),
                    // },
                    // //insert mode
                    // Key::Char('i') => {
                    //     println!("Into insert mode!", );
                    //     //how the heck do we accept text input? We'll work it out...somehow! Byte arrays gonna come into it no doubt
                    // },
                    Key::Esc => {
                        println!("Back to edit mode!");
                        tudu_list.enter_edit_mode();
                    }
                    // Key::Char(c) => {println!("{}", c)}
                    Key::Ctrl(c) => {println!("{}", c)}
                    _ => {}
                }
            }
        };
    };

    Ok(())
}

fn draw_quit_dialog(f: &mut Frame<TermionBackend<RawTerminal<Stdout>>>, size: Rect) {
    let quit_text = Paragraph::new("Really quit?")
        .style(Style::default().fg(Color::Cyan))
        .block(Block::default().borders(Borders::ALL));
    let button_text = Paragraph::new("[Y][N]")
        .style(Style::default().fg(Color::Cyan))
        .block(Block::default().borders(Borders::ALL));
    let area = centered_rect(10, 16, size);
    let quit_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(1),
        ].as_ref())
        .split(area);

    f.render_widget(Clear, area);
    f.render_widget(quit_text, quit_chunks[0]);
    f.render_widget(button_text, quit_chunks[1]);
}
