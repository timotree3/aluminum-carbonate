use serde_derive::Serialize;
use serde_derive::Deserialize;
use std::{fs, io};
use std::path::PathBuf;
use std::ffi::OsString;

#[derive(Serialize, Deserialize)]
struct Database {
    blogs: Vec<Blog>
}

#[derive(Serialize)]
pub struct BlogList {
    items: Vec<String>,
}

#[derive(Serialize)]
pub struct Blog {
    name: String,
    posts: Vec<BlogPost>,
}

#[derive(Serialize)]
pub struct BlogPost {
    title: String,
    content: String,
}

pub struct Error {
    kind: ErrorKind
}

pub enum ErrorKind {
    NotFound,
    Unauthorized,
    SystemFailure,
}

impl BlogPost {
    fn from_directory(title: OsString, mut dir: PathBuf) -> io::Result<Self> {
        let title = title
            .into_string()
            .expect("invalid utf-8 post-names shouldn't be created");
        dir.push("content");
        let content = fs::read_to_string(dir)?;
        Ok(BlogPost {
            title,
            content,
        })
    }
}

pub fn get_blog(name: String) -> Result<Blog, Error> {
    let posts_path = make_posts_path(&name);
    let post_dirs = match fs::read_dir(posts_path) {
        Ok(v) => v,
        Err(e) => match e.kind() {
            io::ErrorKind::NotFound => return Err(Error{kind: ErrorKind::NotFound}),
            _ => return Err(Error{kind: ErrorKind::SystemFailure}),
        }
    };
    let posts = match post_dirs.collect::<io::Result<Vec<fs::DirEntry>>>() {
        Ok(v) => v,
        Err(e) => return Err(Error{kind: ErrorKind::SystemFailure}),
    };
    let posts = posts
        .into_iter()
        .filter_map(|entry|
            BlogPost::from_directory(entry.file_name(), entry.path())
            .ok()
        )
        .collect();
    Ok(Blog {
        name,
        posts,
    })
}

pub fn blog_names() -> Result<Vec<String>, Error> {
    let dir_iter = match fs::read_dir("state/blogs") {
        Ok(i) => i,
        Err(e) => return Err(Error{kind: ErrorKind::SystemFailure}),
    };
    Ok(dir_iter
        .filter_map(|r| r.ok())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect())
}

fn make_posts_path(blog_name: &str) -> PathBuf {
    ["state/blogs", &blog_name, "posts"].iter().collect()
}
