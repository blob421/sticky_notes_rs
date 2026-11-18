use std::collections::HashMap;

use gtk::prelude::*;
use gtk::{Application, ApplicationWindow};
use anyhow::Result;
use gtk::glib::{self, SourceId};
mod db;
use db::{init_db, get_notes, save_note, get_metadata};


struct App {
    notes: HashMap<i32, (String, String)>,
    highest_id: Option<i32>,
    redo_stack: Vec<String>,
    undo_stack: Vec<String>,
    timer_id: Option<SourceId>,
    lock_event_listeners: bool,
    note_ids: Vec<i32>,
    locked: bool,
    current_note_id: i32,
    delete_button : Option<gtk::Button>,
    buffer: Option<gtk::TextBuffer>,
    title: Option<gtk::Entry>

}

impl App {
    fn new() -> Self {
        let (notes, highest_id) = get_notes().unwrap();
        let note_ids = notes.keys().copied().collect();
        let current_note_id = get_metadata().unwrap().unwrap();

        Self {
            notes,
            highest_id: highest_id,
            redo_stack: Vec::new(),
            undo_stack: Vec::new(),
            timer_id: None,
            lock_event_listeners: false,
            note_ids,
            locked: true,
            current_note_id,
            delete_button: None,
            buffer: None,
            title:None,
        }
    }

    fn start(&mut self) -> Result<()> {
        if self.highest_id.is_none() {
           save_note(" ".to_string(), " ".to_string(), true, None)?;

           self.highest_id = Some(1);
        }

        let _app = Application::new(
            Some("StickyNotes"),
            Default::default(),
        );
    


        _app.connect_activate(|app| {
            let window = ApplicationWindow::new(app);
            let display = gtk::gdk::Display::default().unwrap();
            let monitor = display.primary_monitor().unwrap();
            let geometry = monitor.geometry();

            let screen_width = geometry.width();
            let screen_height = geometry.height();
            let y = (screen_height - 700) / 2;
            let x = screen_width - 50;

            self.timer_id = Some(glib::timeout_add_seconds(20, || {}));

            window.set_title("My Notes");
            window.set_default_size(800, 600);
            window.move_(x - 450, y);
            window.show_all();
            window.set_skip_taskbar_hint(true);
            window.set_skip_pager_hint(true);
            window.set_decorated(false);

    });

    Ok(())

    }



    fn get_text(&self) -> String {
       
      if let Some(buffer) = &self.buffer{
        
         let text =  buffer.text(&buffer.start_iter(), &buffer.end_iter(), true).unwrap().to_string();
             
         return text

      }

      else {
        return " ".to_string()      }     
    }


    fn auto_save(&mut self){
        
        let content = self.get_text();

        if let Some(title) = &self.title{
            let title_text = title.text().to_string();
            self.notes.insert(self.current_note_id, (title_text.clone(), content.clone()));

            save_note(content, title_text, false, Some(self.current_note_id));
        }     


        self.timer_id.unwrap().remove();

        self.timer_id = Some(glib::timeout_add_seconds(20, || {self.auto_save ()}));

    }
}


fn main() -> Result<()>{

    // Init database
    if let Err(error) = init_db() {
        eprintln!("Database initialization failed: {error}");
    };


    App::new().start()?;
    
    Ok(())
   
}


