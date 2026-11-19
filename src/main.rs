use std::collections::HashMap;
use std::cell::RefCell;

use gtk::{prelude::*};
use gtk::{Application, ApplicationWindow};
use anyhow::{Result};
use gtk::glib::{self, ControlFlow, SourceId};
mod db;
use std::rc::Rc;

use db::{init_db, get_notes, save_note, get_metadata, delete_note};


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
    title: Option<gtk::Entry>,
    toolbar: Option<gtk::Box>,
    notes_counter: Option<gtk::Button>
   

}

impl App {
    fn new() -> Rc<RefCell<Self>> {
        let (notes, highest_id) = get_notes().unwrap();
        let note_ids = notes.keys().copied().collect();
        let current_note_id = get_metadata().unwrap().unwrap();

        Rc::new(RefCell::new(Self {
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
            toolbar: None,
            notes_counter: None
       
        }))
    }

    fn start(app_state: &Rc<RefCell<Self>>) -> Result<()> {

        

        {
            let mut state = app_state.borrow_mut();


            if state.highest_id.is_none() {
            save_note(" ".to_string(), " ".to_string(), true, None)?;

            state.highest_id = Some(1);
            }
        }


        let _app = Application::new(
            Some("local.blob421.StickyNotes"),
            Default::default(),
        );
    

        let state_clone = Rc::clone(app_state);


        _app.connect_activate(move|app| {
            let window = ApplicationWindow::new(app);

            let display = gtk::gdk::Display::default().unwrap();
            let monitor = display.primary_monitor().unwrap();
            let geometry = monitor.geometry();

            let screen_width = geometry.width();
            let screen_height = geometry.height();
            let y = (screen_height - 700) / 2;
            let x = screen_width - 50;

            let timer_state = Rc::clone(&state_clone);
            
            let timer_id = glib::timeout_add_seconds_local(20, move || {
                // Borrow mutably only for the duration of auto_save()
                timer_state.borrow_mut().auto_save();
                
                // Keep the timer running every 20 seconds
                ControlFlow::Continue
            });

            // Store the SourceId in App
            {
                let mut state = state_clone.borrow_mut();
                state.timer_id = Some(timer_id);
                
                
            }
            Self::load_css();
            Self::make_layout(&state_clone, &window);

            {
                let mut state = state_clone.borrow_mut();
                let note = state.notes.get(&state.current_note_id);

                if let Some(n) = &note{
                    let title = n.0.clone();
                    let content = n.1.clone();
                    state.populate_content(title, content);
                }

       
            }


            window.set_title("My Notes");
            window.set_default_size(450, 700);
            window.move_(x - 450, y);
   
            window.set_skip_taskbar_hint(true);
            window.set_skip_pager_hint(true);
            window.set_decorated(false);

   

            window.show_all();
    });
    _app.run();

    Ok(())

    }


    fn on_text_changed(&mut self){
        if self.lock_event_listeners{
            return 
        }
        let text = self.get_text();

        self.undo_stack.push(text.clone());

        if self.undo_stack.len() > 300 {
            let excess = self.undo_stack.len() - 300;
            self.undo_stack.drain(0..excess);

        }
        let note = self.notes.get(&self.current_note_id);
        
        if let Some(note) = &note{
            let title = note.0.clone();
            self.notes.insert(self.current_note_id, (title, text.clone()));
        }
        
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

            let _ = save_note(content, title_text, false, Some(self.current_note_id));
        }     

    }

    fn make_layout(app_state: &Rc<RefCell<Self>>, window: &ApplicationWindow){


        let bind_state = Rc::clone(app_state);
        let mut state = app_state.borrow_mut();
        
        
        
        Self::bind_keys(bind_state, window);

        

        
    
        let overlay = gtk::Overlay::new();
        window.add(&overlay);

        let paper = gtk::DrawingArea::new();

        paper.connect_draw(|widget, cr| {
            Self::draw_paper(widget, cr);
            glib::Propagation::Proceed
        });

        overlay.add(&paper);
        let content = gtk::Box::new(gtk::Orientation::Vertical, 20);
        content.set_border_width(20);

        overlay.add_overlay(&content);

        let toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 7);

        content.pack_start(&toolbar, false, false, 0);

        let note_pos = state.note_ids.iter().position(|&x| x==state.current_note_id).unwrap();

        let counter_label = format!("{} / {}", note_pos + 1, state.note_ids.len());

        let items = ["⬅️", counter_label.as_str(), "➡️", "+", "🔒", "❌"];

        for (idx, label) in items.iter().enumerate(){

            let button = gtk::Button::new();
            let context = button.style_context();

            button.set_label(label);

            Self::connect_button(app_state, idx, &button, &window);

            

            if idx == 1{
                state.notes_counter = Some(button.clone());
            }

            if idx != 5{
                context.add_class("button_toolbar");
             }

            else if idx == 5{
                state.delete_button = Some(button.clone());
                context.add_class("del_idle");
            }

            

            toolbar.pack_start(&button, false, false, 0);
        
        

            
        };

        state.toolbar = Some(toolbar);

        let title = gtk::Entry::new();
        title.set_placeholder_text(Some("Title..."));
        content.pack_start(&title, false, true, 0);
        state.title = Some(title);

        let scroll = gtk::ScrolledWindow::new(gtk::Adjustment::NONE, 
                                                              gtk::Adjustment::NONE);
        scroll.set_policy(gtk::PolicyType::Automatic, 
                          gtk::PolicyType::Automatic);
        
        let editor = gtk::TextView::new();
        editor.set_wrap_mode(gtk::WrapMode::WordChar);

        let buffer = editor.buffer().clone();


        let buffer_copy = Rc::clone(app_state);
        
         
        let buffer2 = buffer.clone();
        
        buffer.unwrap().connect_changed(move |_| {
            if let Ok(mut state) = buffer_copy.try_borrow_mut() {
                state.on_text_changed();
            }

        });

        state.buffer = buffer2;

        scroll.add(&editor);
        content.pack_start(&scroll, true, true, 0);

        window.connect_destroy(|_| {gtk::main_quit()});
        
    }


    fn bind_keys(app_state:Rc<RefCell<Self>>, window:&ApplicationWindow){
        let accel_group = gtk::AccelGroup::new();

        let state_z = Rc::clone(&app_state);
   
            
        window.add_accel_group(&accel_group);
        
        accel_group.connect_accel_group(*gtk::gdk::keys::constants::Z,
            gtk::gdk::ModifierType::CONTROL_MASK, 
            gtk::AccelFlags::VISIBLE, move |_group, _acceleratable, _keyval, _modifier| {
                println!("CTRL+Z PRESSED");
                state_z.borrow_mut().undo();
                true
         }
        );
        let state_y = Rc::clone(&app_state);

        accel_group.connect_accel_group(*gtk::gdk::keys::constants::Y,
            gtk::gdk::ModifierType::CONTROL_MASK, 
            gtk::AccelFlags::VISIBLE, move |_group, _acceleratable, _keyval, _modifier| {
                let _ = state_y.borrow_mut().redo();
                true
         }
        );
 


    }

    fn undo(&mut self){
        let undo_len = self.undo_stack.len();
        println!("Undo stack: {:?}",self.undo_stack); 
        if undo_len == 1 {return}

        if undo_len > 0 {
            self.lock_event_listeners = true;
            self.redo_stack.push(self.undo_stack.pop().unwrap());

            let text = self.undo_stack.last().unwrap();
            
            if let Some(buff) = &self.buffer {
               buff.set_text(text);
            }
            self.lock_event_listeners = false;
        }
    }
        
        

    fn redo(&mut self){
        let redo_len = self.redo_stack.len();
        if redo_len > 0 {
            self.lock_event_listeners = true;
            let text = self.redo_stack.pop().unwrap();

            self.undo_stack.push(text.clone());

            if let Some(buff) = &self.buffer{
                buff.set_text(&text)
            }
            self.lock_event_listeners = false;

        }
        
        
    }
    fn connect_button(app_state:&Rc<RefCell<Self>>, idx:usize, button:&gtk::Button, window: &ApplicationWindow){

        match idx{
            0 | 2 => {
              
                let direction = if idx == 0 {"l"} else {"r"};

                let state = Rc::clone(app_state); // Old handle could disappear, make a new one

                button.connect_clicked(move |_| {                        // move : take ownership of state
                        state.borrow_mut().switch_note(direction.to_string(), None, false);
                    });
            }

            3 => {
                let state = Rc::clone(app_state);
                button.connect_clicked(move |_| {
                    let _ = state.borrow_mut().create_note();
                  
                });
            }

            4 => {
                let state = Rc::clone(app_state);
                let win = window.clone();
                button.connect_clicked(move |_| {
                    state.borrow_mut().toggle_lock(&win);
                });                

            }

            5 => {
                let state = Rc::clone(app_state);
                button.connect_clicked(move |_| {
                    let _ = Self::handle_delete(&state, false);
                });                

            }
            _ => {

            }
        }

    }



    fn create_note(&mut self) -> Result<()>{
       self.auto_save();

       let last_id = self.note_ids.last();

       if last_id.is_some(){
          let next_id = last_id.unwrap() + 1;
          
          self.notes.insert(next_id, (" ".to_string(), " ".to_string()));
          self.note_ids.push(next_id);
          self.current_note_id = next_id;

          if let Some(title) = &self.title {
            title.set_text(" ");
          }

          if let Some(textarea) = &self.buffer {
            textarea.set_text(" ")
          }

          if let Some(counter) = &self.notes_counter {
                let ids_length = self.note_ids.len() as i32;
                counter.set_label(&format!("{} / {}", ids_length, ids_length));

                save_note(" ".to_string(), " ".to_string(), true, Some(self.current_note_id))?;
          }
        

       }

     self.undo_stack = vec!["".to_string()];
     self.redo_stack.clear();

     Ok(())

    }

    fn toggle_lock(&mut self, window: &ApplicationWindow){

        self.locked = !self.locked;

        if self.locked{
            window.set_decorated(false);
        }
  
        else{
           window.set_decorated(true);
        }

    }
    fn switch_note(&mut self, direction:String, position: Option<i32>, delete:bool){

       let pos;

       if position.is_none(){
          pos = self.note_ids.iter().position(|&x| x==self.current_note_id).unwrap() as i32;
       }
       else {
          pos = position.unwrap();
       }

       if !delete {
           self.auto_save();
       }

       let offset = if direction == "l" {-1} else {1};

       let notes_len = self.note_ids.len() as i32;

       let new_position = (pos + offset + notes_len) % notes_len;

       let current_note_id = self.note_ids[new_position as usize];

       let new_note = self.notes.get(&current_note_id);

       self.current_note_id = current_note_id;
       
       if let Some(counter) = &self.notes_counter{
        
             counter.set_label(&format!("{} / {}", new_position + 1, self.note_ids.len()));
       }

       if let Some(note) = &new_note{
           let new_title = note.0.clone();
           let new_content = note.1.clone();
           self.populate_content(new_title, new_content);
       }
       

        
        
      
    }

    fn handle_delete(app_instance: &Rc<RefCell<Self>>, reset:bool) -> Result<()>{

        let state_clone = Rc::clone(&app_instance);

        {
            let mut state = state_clone.borrow_mut();

            if let Some(button) = &state.delete_button{

                let context = button.style_context();

                if reset {

                    if context.has_class("del_clicked"){
                        context.remove_class("del_clicked");
                        context.add_class("del_idle");
                    }
    
                }

                else if !context.has_class("del_clicked"){
                        context.add_class("del_clicked");
                        context.remove_class("del_idle");

                        let clone = Rc::clone(&state_clone);
                        glib::timeout_add_seconds_local(6, move || {
                            let _ = Self::handle_delete(&clone, true);
                            ControlFlow::Break
                        });
                        
                }
                else{
                    context.add_class("del_idle");
                    context.remove_class("del_clicked");
                    if state.note_ids.len() > 1 {
                         state.delete_note()?;
                    }
          
                }


            } 
            
        }
     Ok(())

    }
    fn delete_note(&mut self) -> Result<()> {
        
        let position = self.note_ids.iter().position(|&x| x == self.current_note_id); 

        self.notes.remove(&self.current_note_id);

        delete_note(self.current_note_id)?;

        if let Some(p) = position{
            self.note_ids.remove(p);
            let upos = p as i32;
            self.switch_note("l".to_string(), Some(upos), true);
        }

      Ok(())
    }


    fn populate_content(&mut self, title:String, content:String){
        self.lock_event_listeners = true;

        if let Some(title_widget) = &self.title {
          title_widget.set_text(&title);
        }
        if let Some(textarea) = &self.buffer {
            textarea.set_text(&content);
        }
        self.redo_stack.clear();
        self.undo_stack = vec![content];
        self.lock_event_listeners = false;

    }


    fn draw_paper(widget: &gtk::DrawingArea, cr: &gtk::cairo::Context) {
        let allocation = widget.allocation();
        let width = allocation.width() as f64;
        let height = allocation.height();

        // Yellow paper
        cr.set_source_rgb(0.965, 0.910, 0.660);
        let _ = cr.paint();

        // Horizontal ruled lines
        cr.set_source_rgba(0.55, 0.48, 0.30, 0.25);
        cr.set_line_width(1.0);

        for y in (176..height).step_by(28) {
            let y_f64 = y as f64;
            cr.move_to(20.0, y_f64);
            cr.line_to(width - 20.0, y_f64);
            let _ = cr.stroke();
        }
    }

    fn load_css() {
        let provider = gtk::CssProvider::new();

        provider
            .load_from_data(include_bytes!("styles.css"))
            .expect("Failed to load CSS");


        if let Some(screen) = gtk::gdk::Screen::default() {
            gtk::StyleContext::add_provider_for_screen(
                &screen,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    }
}


fn main() -> Result<()>{

    // Init database
    if let Err(error) = init_db() {
        eprintln!("Database initialization failed: {error}");
    };


    let app_state = App::new();
    App::start(&app_state)?;
    
    Ok(())
   
}


