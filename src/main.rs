use std::env;
use std::fs::File;
use std::path::Path;
use std::panic;
use std::ffi::OsStr;
use serde_json::Value;
use std::io::Read;

#[macro_use]
extern crate lazy_static;
extern crate tera;

use tera::*;

fn main() {
    println!("Hello, world!");

    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);

    match args.get(1) {
        None => {
            panic!("expected argument path to the root content folder of a Ghost backup");
        },
        Some(file_path) => {
            parse_root(file_path);
        },
    }
}

fn get_database(path: &String) -> Option<Value> {
    let content_folder = Path::new(path);
    if !content_folder.exists() || !content_folder.is_dir() {
        panic!("the specified file path {:?} is not a folder.", path);
    }

    let data_folder = content_folder.join("data");
    if !data_folder.exists() || !data_folder.is_dir() {
        panic!("the specified file path {:?} does not contain /data folder.", path);
    }

    for data_file in data_folder.read_dir().expect("read_dir call failed for /data") {
        if let Ok(data_file) = data_file {
            if let Some(extension) = data_file.path().extension().and_then(OsStr::to_str) {
                if extension == "json" {
                    let mut data = String::new();
                    if let Ok(mut file) = File::open(data_file.path()) {
                        if let Ok(_) = file.read_to_string(&mut data) {
                            let v: Value = serde_json::from_str(&data).expect("failed to read json");
                            return Some(v);
                        }
                    }
                }
            }
        }
    }

    return None;
}


lazy_static! {
    pub static ref TERA: tera::Tera = {
        let mut tera = tera::compile_templates!("templates/*.html");
        tera.autoescape_on(vec![]);
        tera
    };
}

fn parse_root(path: &String) {

    if let Some(json) = get_database(path) {
        if let Some(posts) = json["data"]["posts"].as_array() {
            for post in posts {
                println!("found post {:?}", post["title"]);
                let title = post["title"].as_str();

                let mut context = Context::new();
                context.insert("post", post);

                match TERA.render("post.html", &context) {
                    Ok(result) => {println!("rendered: {:?}", result);},
                    Err(err) => {println!("failed to render: {:?}", err);},
                }
            }
        }
    }
}
