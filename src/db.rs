
use rusqlite::Connection;
use directories::ProjectDirs;
use std::fs;
use anyhow::Result;
use std::sync::LazyLock;
use std::collections::HashMap;


static DIRS: LazyLock<ProjectDirs> = LazyLock::new(|| {
    ProjectDirs::from("local", "blob421", "StickyNotes").unwrap()});



//println!("{:?}", dirs.data_dir()); 
//println!("{:?}", dirs.config_dir());
//println!("{:?}", dirs.cache_dir());


pub fn init_db() -> Result<()> {

    let db_path = DIRS.data_dir();

    fs::create_dir_all(db_path)?;
    println!("{:?}", db_path.join("db.sqlite"));

    let conn = Connection::open(db_path.join("db.sqlite"))?;
    
    conn.execute("CREATE TABLE IF NOT EXISTS notes (
                                                    id INTEGER PRIMARY KEY,
                                                    title TEXT,
                                                    content TEXT
                                                    )", [])?;
    
    conn.execute("CREATE TABLE IF NOT EXISTS metadata(last_idx INTEGER)", [])?;

    Ok(())

}

pub fn delete_note(id:i32) -> Result<()> {
    
    let conn: Connection = Connection::open(DIRS.data_dir().join("db.sqlite"))?;
    conn.execute("DELETE FROM notes WHERE id=?", (id,))?;

    Ok(())
}


pub fn get_notes() -> rusqlite::Result<(HashMap<i32, (String, String)>, Option<i32>)> {
    let conn = Connection::open(DIRS.data_dir().join("db.sqlite"))?;

    let mut stmt = conn.prepare("SELECT * FROM notes ORDER BY id ASC")?;

    let data = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i32>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;

    let mut parsed = HashMap::new();
    let mut highest_id = None;

    for note in data {
        let (id, title, content) = note?;

        highest_id = Some(id);
        parsed.insert(id, (title, content));
    }

    if parsed.is_empty() {
        parsed.insert(1, (" ".to_string(), " ".to_string()));
        return Ok((parsed, None));
    }

    Ok((parsed, highest_id))
}


pub fn save_note(content:String, title:String, create:bool, id:Option<i32>) -> Result<()> {

    let conn: Connection = Connection::open(DIRS.data_dir().join("db.sqlite"))?;

    if id.is_some() && !create{
       conn.execute("UPDATE notes SET title=?,content=? WHERE id=?", (title, content, id))?;
       conn.execute("UPDATE metadata SET last_idx=? WHERE rowid=?", (id, 1))?;
    }


    else if id.is_some() && create {
       conn.execute("INSERT OR IGNORE INTO notes(id, title, content) VALUES(?,?,?)",(id, title, content))?; 

    }

    else if id.is_none(){
      conn.execute("INSERT OR IGNORE INTO notes(id, title, content) VALUES(?,?,?)",(1, title, content))?;  

    }


    Ok(())

}


pub fn get_metadata() -> Result<Option<i32>> {
    let conn = Connection::open(DIRS.data_dir().join("db.sqlite"))?;

    let data = conn.query_row(
        "SELECT last_idx FROM metadata LIMIT 1",
        [],
        |row| row.get::<_, i32>(0),
    );

    match data {

        Ok(id) => Ok(Some(id)),

        Err(rusqlite::Error::QueryReturnedNoRows) => {
            conn.execute(
                "INSERT INTO metadata(last_idx) VALUES(?)",
                [1],
            )?;

            Ok(Some(1))
        },

        Err(e) => Err(e.into()),
    }
}