use std::fs::File;
use std::io::BufReader;
use dotenvy::dotenv;
use serde::Deserialize;
use serde_json::from_reader;
use std::process::Command;
use std::env;


#[derive(Deserialize)]
struct Course {
    faculty: i16,
    id: String, 
    name: String
}

fn main() -> Result<(), ()> { 
    let file = File::open("src/data/Courses.json").expect("Failed to open Courses.json");
    let reader = BufReader::new(file);
    let courses: Vec<Course> = from_reader(reader).expect("Failed to parse JSON file");

    // Step 2: Generate SQL inserts
    let mut inserts = String::new();
    for course in courses {
        inserts.push_str(&format!(
            "INSERT INTO courses (Course_ID, Course_Name, Course_Faculty) VALUES ('{}', '{}', {});\n",
            course.id, course.name, course.faculty
        ));
    }
    
    dotenv().ok();
 
    Command::new("psql")
        .arg("-d")
        .arg(env::var("DATABASE_URL").expect("DATABASE_URL must be set"))
        .arg("-c")
        .arg(&inserts)
        .status()
        .expect("Failed to run database inserts");

    Ok(())
}