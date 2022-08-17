use std::error::Error;
use std::io;
use std::io::{Stdout, stdout};

use chrono::prelude::*;
use clap::{Command, ArgMatches, Arg};
use log::{debug, error, LevelFilter};
//use termion::{clear, raw::IntoRawMode};
//use termion::event::Key;
//use termion::raw::RawTerminal;
// use tui::backend::TermionBackend;
use crossterm::event::{read, Event, KeyCode, KeyEvent};
use crossterm::{ExecutableCommand, execute, terminal};
use crossterm::event::EnableMouseCapture;
use crossterm::terminal::{ClearType, EnterAlternateScreen, enable_raw_mode};
use tui::{Frame, backend::CrosstermBackend, Terminal};
use tui::layout::{Constraint, Direction, Layout, Rect, Alignment};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, BorderType};

use model::InputMode;

// use crate::events::{Event, Events};
use crate::model::{ MoveDirection, RutuduList};

use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::{Config};
use log4rs::config::{Appender, Root};

mod model;
mod db;
mod export;

use num_traits::cast::ToPrimitive;

// const DATE_FMT: &str = "%Y%m%d%H%M%s";
const DATE_FMT: &str = "%Y%m%d";

fn init_args() -> ArgMatches {
    Command::new("Rutudu Todo List")
        .version("1.0")
        .author("FOOM")
        .about("Todo List, Terminal style, Rust vibes")
        .arg(Arg::new("list_name")
            .value_name("list_name")
            .help("Name of list, will default to 'rutudu$DATE.rtd' if not supplied. Name of sqlite file.")
            // .about("Name of list, will default to 'rutudu$DATE.rtd' if not supplied. Name of sqlite file.")
            .index(1)
        .required(false))
        .arg(Arg::new("verbose")
            .long("verbose")
            .short('v')
            .help("All the info"))
        .arg(Arg::new("time_tracking_file")
            .short('t')
            .long("time-track-file")
            .takes_value(true)
            .help("If you run with time tracking and want to specify a location separate from the sqlite file used by list"))
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
                Constraint::Percentage(30),
                Constraint::Min(min_horizontal),
                Constraint::Percentage(60),
            ].as_ref(),
        ).split(v[1])[1]

}

fn get_default_list_name()->String{
    debug!("No name arg passed...");
    let today = Utc::now().format(DATE_FMT);
    format!("./rutudu{}.rtd", today)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = init_args();
    init_logger(args.is_present("verbose"));
    let default_name = get_default_list_name();
    // let list_name = args.value_of("list_name").unwrap_or(&default_name);
    let list_name = args.value_of("list_name").unwrap_or(&default_name);
    #[cfg(feature="clockrust")]
    let tracking_name:Option<&str> = if let Some(tn) = args.value_of("time_tracking_file"){
        Some(tn)
    }else{
        None
    };

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    // println!("{}", clear::All);
    //display first/appropriate todolist
    //display all cell and it's children (recursively)
    //display add new list
        //we want the screen to be cleared

    stdout.execute(terminal::Clear(ClearType::All))?;
    //like in vim, 2 modes, edit and insert
        //few more modes now, we in state machine territory
    // let mut edit_mode = true;
    let mut tudu_list = RutuduList::default();
    tudu_list.set_file_path( list_name);
    tudu_list.open_list(list_name);
    debug!("We think our list is unsaved = {}", tudu_list.unsaved.clone());

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
                    .fg(tudu_list.highlight_color()))
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
                InputMode::Open | InputMode::Import =>  draw_open_dialog(&mut tudu_list,f),
                InputMode::Edit =>  {},
                #[cfg(feature ="clockrust")]
                InputMode::PrintReport => draw_print_report_dialog(&mut tudu_list, f),
                InputMode::DisplaySuccess => draw_popup("Success!", f),
            }
        }).unwrap();

        // if let Event::Input(input) = events.next()? {
        if let Ok(Event::Key(input)) = read()?{
            match tudu_list.input_mode {
                InputMode::Edit => match input {
                    KeyEvent::KeyCode::Char('q') => tudu_list.enter_quit_mode(),
                    KeyCode::Char('S') => tudu_list.enter_save_mode(),
                    KeyCode::Char('s') => tudu_list.save(),
                    KeyCode::Char('o') => tudu_list.enter_open_mode(),
                    KeyCode::Char('I') => tudu_list.enter_import_mode(),
                    KeyCode::Char('x') => tudu_list.toggle_selected_item_completion_status(),
                    KeyCode::Char('X') => tudu_list.toggle_selected_item_and_children_completion_status(),
                    KeyCode::Char('d') => tudu_list.move_item(MoveDirection::Down),
                    KeyCode::Char('u') => tudu_list.move_item(MoveDirection::Up),
                    KeyCode::Char('>') | KeyCode::Char('i') => tudu_list.move_item(MoveDirection::In),
                    KeyCode::Char('<') => tudu_list.move_item(MoveDirection::Out),

                    KeyCode::Char('h') | KeyEvent::Left => tudu_list.collapse_selected(),
                    KeyCode::Char('j') | KeyCode::Down => tudu_list.down(),
                    KeyCode::Char('k') | KeyCode::Up =>  tudu_list.up(),
                    KeyCode::Char('l') | KeyCode::Right =>  tudu_list.expand_selected(),

                    KeyCode::Char('m') => tudu_list.mark_selected_item(),
                    KeyCode::Ctrl('x') => match tudu_list.export_as_markup(){
                    Ok(s) => debug!("Successfully exported as markup"),
                        Err(why) => error!("Failed to export as markup {}", why),
                }

                    KeyCode::Delete | KeyCode::Backspace => tudu_list.delete_selected(),
                    //ctrl+e ...really? why no ctrl+backspace - guess cos it's a weird hex code not a char...
                    KeyCode::Ctrl('e') => tudu_list.erase_selected(),//does not preserve children

                    KeyCode::Char('a') => tudu_list.enter_insert_mode(InputMode::InsertSibling),
                    KeyCode::Char('A') => tudu_list.enter_insert_mode(InputMode::InsertAtRoot),
                    KeyCode::Ctrl('a') => tudu_list.enter_insert_mode(InputMode::InsertChild),
                    KeyCode::Alt('a') => tudu_list.enter_insert_mode(InputMode::InsertParent),

                    #[cfg(feature ="clockrust")]
                    KeyCode::Ctrl('t') => if let Some(track_file) = tracking_name{
                        tudu_list.track_time(Some(track_file));
                    }else{
                        let fp = tudu_list.file_path();
                        let tf = Some(&fp[..]);
                        tudu_list.track_time(tf);
                    },


                    #[cfg(feature="clockrust")]
                    KeyCode::Alt('t') => tudu_list.enter_print_tracking_report_mode(),

                    _ => {}
                },
                InputMode::InsertAtRoot | InputMode::InsertChild | InputMode::InsertParent | InputMode::InsertSibling => match input {

                    //what's a better key combo? ctrl+[ does weird things...
                    //terminal doesn't support ctrl+\n, ctrl/shift don't modify the key being pressed dammit
                    //alt+\n just does not seem to work?
                    KeyCode::Ctrl('n') =>  tudu_list.add_input_text_as_item_to_list(),//any way to combine with bottom row? so far not found....
                    KeyCode::Alt(c) => if c as u32 == 13 {
                        debug!("Alt was pressed with enter!!");
                        tudu_list.add_input_text_as_item_to_list();
                    }
                    else{
                        debug!("We pressed alt+{}", c);
                        debug!("Ascii val == {}", c as u32);
                    }
                    KeyCode::Backspace => tudu_list.remove_character(),
                    KeyCode::Left => tudu_list.cursor_left(),
                    KeyCode::Right => tudu_list.cursor_right(tudu_list.file_path().len()),
                    KeyCode::Char(c) => tudu_list.add_character(c),//tudu_list.current_item.push(c),
                    KeyCode::Esc => tudu_list.enter_edit_mode(),
                    // KeyCode::Char(c) => {println!("{}", c)}
                    KeyCode::Ctrl(c) => { println!("{}", c) }
                    _ => {}
                },
                InputMode::Save => match input{
                    KeyCode::Ctrl('n') => {
                        db::save_list(&tudu_list).unwrap();
                        tudu_list.mark_saved();
                    }
                    KeyCode::Char(c) => if '\n' == c {
                        db::save_list(&tudu_list).unwrap();
                        tudu_list.mark_saved();
                    }else{
                        tudu_list.add_save_input_char(c);
                    },
                    KeyCode::Ctrl('\n') => {//how can I combine with the above?
                        db::save_list(&tudu_list).unwrap();
                        tudu_list.mark_saved();

                    }
                    KeyCode::Left => tudu_list.cursor_left(),
                    KeyCode::Right => tudu_list.cursor_right(tudu_list.file_path().len()),
                    KeyCode::Backspace => tudu_list.remove_save_file_char(),
                    KeyCode::Esc => tudu_list.enter_edit_mode(),
                    _ => {}
                },
                InputMode::Open => match input{//allow moving up and down to select
                    KeyCode::Char('j') | KeyCode::Down => tudu_list.open_file_down(),
                    KeyCode::Char('k') | KeyCode::Up => tudu_list.open_file_up(),
                    KeyCode::Char('l') | KeyCode::Right | KeyCode::Char('\n') | KeyCode::Ctrl('n') =>tudu_list.load_list_from_file_dialog(),
                    KeyCode::Esc => tudu_list.enter_edit_mode(),
                    _ => {},
                },
                InputMode::Import => match input{//allow moving up and down to select
                    KeyCode::Char('j') | KeyCode::Down => tudu_list.open_file_down(),
                    KeyCode::Char('k') | KeyCode::Up => tudu_list.open_file_up(),
                    KeyCode::Char('l') | KeyCode::Right | KeyCode::Char('\n') | KeyCode::Ctrl('n') =>tudu_list.import_list_from_file_dialog(),
                    KeyCode::Esc => tudu_list.enter_edit_mode(),
                    _ => {},
                },
                InputMode::Quit => match input {
                    KeyCode::Char('y') | KeyCode::Char('\n') => {

                        let mut stdout = io::stdout();
                        stdout.execute(terminal::Clear(ClearType::All))?;
                        break;
                    },
                    KeyCode::Char('n') | KeyCode::Esc => tudu_list.enter_edit_mode(),
                    _ => {}
                }
                #[cfg(feature ="clockrust")]
                InputMode::PrintReport => match input{
                    // KeyCode::Char('\n') => tudu_list.create_report(),//wtf, why did this not work?
                    KeyCode::Char(c) => if c=='\n'{//won't pick it up if it's standalone for some reason, the \n
                        tudu_list.create_report();
                    }else{
                        tudu_list.add_char_to_report_dialog(c);
                    }
                    KeyCode::Left => tudu_list.cursor_left(),
                    KeyCode::Right => tudu_list.cursor_right(tudu_list.report_path().len()),
                    KeyCode::Backspace => tudu_list.remove_char_from_report_dialog(),
                    KeyCode::Esc => tudu_list.enter_edit_mode(),
                    _ => {},
                }

                InputMode::DisplaySuccess => match input{
                    //whatever you do, go back to edit mode
                    _ => tudu_list.enter_edit_mode(),
                }
            }
        };
    };

    Ok(())
}

fn draw_popup(txt: &str, f: &mut Frame<Terminal<CrosstermBackend<Stdout>>>) {
    let size = f.size();
    let text = Paragraph::new(txt)
        .style(Style::default().fg(Color::Cyan))
        .block(Block::default().borders(Borders::ALL));
    let area = little_popup(20, 3, size);
    f.render_widget(Clear, area);
    // f.render_widget(quit_text, quit_chunks[0]);
    f.render_widget(text, area);
}
fn show_new_item_input(tudu_list: &mut RutuduList, f: &mut Frame<CrosstermBackend<Terminal<Stdout>>>) {
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

fn draw_quit_dialog(f: &mut Frame<CrosstermBackend<Terminal<Stdout>>>) {
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

///Draw dialog that allows saving of the tudulist
/// Allows changing of the filename
fn draw_save_dialog(tudu_list: &mut RutuduList, f: &mut Frame<CrosstermBackend<Terminal<Stdout>>>){
    let rect = f.size();
    let save_text = Paragraph::new(tudu_list.file_path())
        .style(Style::default().fg(Color::Cyan))
        .block(Block::default().borders(Borders::ALL).title("[S]ave?"));
    let area = little_popup(40,5, rect);

    f.render_widget(Clear,area);
    f.render_widget(save_text, area);
    // tudu_list.cursor_position[0] =
    f.set_cursor(area.x as u16 + tudu_list.cursor_position[0] as u16 +1, area.y as u16 + tudu_list.cursor_position[1] );
}

///Draw dialog that allows printing of time tracking report.
/// Allows entering of time tracking report name
#[cfg(feature="clockrust")]
fn draw_print_report_dialog(tudu_list: &mut RutuduList, f: &mut Frame<TermionBackend<RawTerminal<Stdout>>>) {
   let rect = f.size();
    let report_path = tudu_list.report_path();
    let report_path_text = Paragraph::new(report_path)
        .style(Style::default().fg(Color::Cyan))
        .block(Block::default().borders(Borders::ALL).title("Save report?"));
    let area = little_popup(70, 5, rect);

    f.render_widget(Clear, area);
    f.render_widget(report_path_text, area);

    f.set_cursor(area.x as u16 + tudu_list.cursor_position[0] as u16, area.y as u16 + tudu_list.cursor_position[1] );


}

///Draw dialog with a display of the files in the current directory
fn draw_open_dialog(tudu_list: &mut RutuduList, f: &mut Frame<CrosstermBackend<Terminal<Stdout>>>) {
    tudu_list.scan_files_once();

    // debug!("Trying to draw open dialog");
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
