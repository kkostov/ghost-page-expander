use std::env;
use std::fs::File;
use std::path::Path;
use std::panic;
use std::ffi::OsStr;
use serde_json::Value;
use std::io::{Read, Write};
use std::fs;

extern crate tera;
use tera::*;


extern crate chrono;
use chrono::{DateTime, NaiveDateTime, Utc};

extern crate htmlstream;


fn main() {

    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);

    match args.get(1) {
        None => {
            panic!("expected argument path to the root content folder of a Ghost backup");
        },
        Some(file_path) => {
            match args.get(2) {
                None => {
                    panic!("expected second argument glob pattern path to the template folder e.g. './templates/*.html'");
                },
                Some(template_path) => {
                    match args.get(3) {
                        None => {
                            panic!("expected third argument path to the output root folder");
                        },
                        Some(output_path) => {
                            parse_root(file_path, template_path, output_path);
                        },
                    }
                },
            }
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
                    println!("importing file {:?}", data_file.path());
                    let mut data = String::new();
                    if let Ok(mut file) = File::open(data_file.path()) {
                        if let Ok(_) = file.read_to_string(&mut data) {
                            let v: Value = serde_json::from_str(&data).expect("failed to read json");


                            return Some(v["db"]
                                .as_array()
                                .expect("invalid json format, expected a root 'db' array.")
                                [0].clone());
                        }
                    }
                }
            }
        }
    }

    return None;
}


static OUTPUT_PAGE_FILENAME: &str = "index.html";
static TEMPLATE_NAME_POST: &str = "post.html";




/// Reads a ghost back-up folder structure and renders all published posts
fn parse_root(path: &String, templates_path: &String, output_path: &String) {

    if let Some(json) = get_database(path) {
        if let Some(posts) = json["data"]["posts"].as_array() {
            println!("expanding {} posts", posts.len());
            for post in posts {

                if let Some(timestamp) = post["published_at"].as_i64() {

                    println!("working post");

                    let post_folder = get_content_folder(timestamp);
                    let post_folder_path = Path::new(output_path).join(post_folder);


                    let mut context = Context::new();
                    context.insert("post", post);

                    let published_at = timestamp / 1000;
                    context.insert("published_at", &published_at); // so the chrono formatters in the template can work


                    let mut content = render_content(templates_path, TEMPLATE_NAME_POST, &context);

                    let links = get_img_links(&content);
                    let content_path = Path::new(path);
                    move_img_links(&mut content, &links, &post_folder_path, &content_path);

                    write_content(&content, &post_folder_path);

                } else {
                    println!("skipping unpublished post {:?} {:?}", post["id"], post["title"]);
                }
            }
        }
    }
}

/// Given a JavaScript timestamp (millis Unix epoch time) returns a post folder name
fn get_content_folder(timestamp: i64) -> String {
    let timestamp_sec = timestamp / 1000;
    let naive = NaiveDateTime::from_timestamp(timestamp_sec, 0);
    let datetime = DateTime::<Utc>::from_utc(naive, Utc);
    datetime.format("post-%Y-%m-%d").to_string()
}

/// Generates a content string by inflating the specified template using the provided context data
fn render_content(
    templates_path: &String,
    template_name: &str,
    tera_context: &Context) -> String {

    let mut tera: Tera = compile_templates!(templates_path);
    tera.autoescape_on(vec![]);

    match tera.render(template_name, tera_context) {
        Ok(content) => {
            return content;
        },
        Err(err) => {panic!("failed to render: {:?}", err);},
    }
}

/// Writes the content string to disk
fn write_content(content: &String, write_dir_path: &Path) {
    // ensure the folder doesn't exist (and it's empty)
    if write_dir_path.exists() {
        fs::remove_dir_all(write_dir_path).expect("failed to delete output path");
    }

    // create the folder and write out the index
    match fs::create_dir_all(write_dir_path) {
        Ok(_) => {
            let output_file_path = write_dir_path.join(OUTPUT_PAGE_FILENAME);
            match File::create(output_file_path) {
                Ok(mut file) => {
                    file.write_all(content.as_bytes()).expect("failed to write to file");
                },
                Err(err) => {
                    panic!("failed to create file {:?} in output path {:?} {:?}", OUTPUT_PAGE_FILENAME, write_dir_path, err);
                },
            }

        },
        Err(err) => {
            panic!("failed to create output path {:?} {:?}", write_dir_path, err);
        },
    }
}

static HTML_TAG_IMG: &str = "img";
static HTML_ATTRIBUTE_IMG_SOURCE: &str = "src";

#[derive(Debug)]
struct MediaLink {
    /// Position of the beginning of the html tag, e.g. that '<' of '<img src...'
    position: htmlstream::Position,
    url: String
}

fn get_img_links(content: &String) -> Vec<MediaLink> {
    let mut results: Vec<MediaLink> = vec![];

    for (pos, tag) in htmlstream::tag_iter(content) {
        if tag.name == HTML_TAG_IMG {
            if let Some((_, attr_src)) =
            htmlstream::attr_iter(&tag.attributes)
                .find(|(_, attr)| attr.name == HTML_ATTRIBUTE_IMG_SOURCE) {

                let media_link = MediaLink { position: pos, url: attr_src.value };
                println!("media link {:?}", media_link);
                results.push(media_link);
            }
        }
    }

    results
}

/// Copies or downloads external media into the post folder path and updates the content
/// so that the source path point to the new locations
fn move_img_links(content: &mut String, links: &Vec<MediaLink>, write_dir_path: &Path, content_path: &Path) {
    let mut offset: i64 = 0;

    for (i, img) in links.iter().enumerate() {
        if img.url.starts_with("http") {
            // download
        } else if img.url.starts_with("/content") {
            // in html the url is absolute, adding a '.' to make it relative
            let img_url = format!(".{}", &img.url);

            // in html the url includes the "/content" prefix
            let original = content_path.parent().unwrap().join(&img_url).canonicalize().unwrap();


            if let Some(extension) = original.extension().and_then(OsStr::to_str) {
                let new_name = format!("pic{}.{}", i, extension.to_lowercase());
                let new_tag = format!("<img src=\"{}\" alt=\"\" />", format!("./{}", new_name));


                let start = img.position.start as i64 + offset;
                let end = img.position.end as i64 + offset;

                offset = offset + new_tag.len() as i64 - (img.position.end - img.position.start) as i64;

                // update the content
                println!("{} start: {}, end: {}", i, start, end);
                println!("replacing '{}'", &content[start as usize..end as usize]);

                content.replace_range(
                    start as usize..end as usize,
                    new_tag.as_str());

                // copy the file
                let copy = write_dir_path.join(new_name);
                println!("copying {:?} to {:?}", original, copy);
                fs::copy(original, copy).expect("failed to copy image");
            }
        } else {
            println!("incompatible media link source {:?}. Leaving unchanged.", img.url);
        }
    }

}
