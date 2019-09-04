#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use rocket::http::ContentType;
use rocket::request::FlashMessage;
use rocket::response::content::Content;
use rocket::response::{Flash, Redirect};
use std::fmt::Display;
use std::fs::File;
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};

fn path_for_blog(name: &str) -> PathBuf {
    let mut path = "state/blogs/".to_string();
    base64::encode_config_buf(name, base64::Config::new(base64::CharacterSet::UrlSafe, false), &mut path);
    dbg!(path.into())
}

fn name_from_path<A: AsRef<Path>>(path: A) -> String {
    let encoded = path
        .as_ref()
        .file_name()
        .expect("name_from_path called with invalid blog path")
        .to_str()
        .expect("base64 encoded blog name isn't valid utf8");
    String::from_utf8(base64::decode_config(encoded, base64::Config::new(base64::CharacterSet::UrlSafe, false)).expect("blog name isn't valid base64")).expect("blog name isn't valid utf8")
}

#[derive(FromForm)]
struct BlogMeta {
    name: String,
    description: Option<String>,
}

impl Display for BlogMeta {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "{}: {}", &self.name, self.description.as_ref().unwrap_or(&"".to_string()))
    }
}

struct BlogList {
    blogs: Vec<BlogMeta>,
}

impl Display for BlogList {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "<ul>")?;
        for meta in &self.blogs {
            write!(f, "<li><a href=\"{}\">{}</a></li>", uri!(blog_home: &meta.name), meta)?;
        }
        write!(f, "</ul>")
    }
}

#[get("/")]
fn index() -> Content<&'static str> {
    Content(ContentType::HTML, include_str!("index.html"))
}

#[get("/newblog")]
fn new_blog_form(flash: Option<FlashMessage>) -> Content<String> {
    let flash_result = flash
        .map(|msg| format!("{}: {}", msg.name(), msg.msg()))
        .unwrap_or_else(|| "".to_string());

    Content(
        ContentType::HTML,
        format!(include!("newblog.html"), flash_result),
    )
}

#[post("/newblog", data = "<submission>")]
fn new_blog_accept(
    submission: rocket::request::Form<BlogMeta>,
) -> Result<Redirect, Flash<Redirect>> {
    // create the folder for the blog data
    let blog_path = path_for_blog(&submission.name);
    match std::fs::create_dir(&blog_path) {
        Ok(()) => {}
        Err(ref e) if e.kind() == ErrorKind::AlreadyExists => {
            // name taken
            // return Err()...
            return Err(Flash::error(
                Redirect::to(uri!(new_blog_form)),
                "name taken",
            ));
        }
        Err(e) => panic!("unable to create blog dir for reason: {:?}", e),
    }
    // create the empty folder for the posts
    match std::fs::create_dir(blog_path.join("posts")) {
        Ok(()) => {}
        Err(ref e) if e.kind() == ErrorKind::NotFound => {
            // error to user with internal error occured, try again in a few seconds
            return Err(Flash::warning(
                Redirect::to(uri!(new_blog_form)),
                "internal error occured, try again in a few seconds",
            ));
        }
        Err(e) => panic!("unable to create posts folder for reason: {:?}", e),
    }

    let desc = if let Some(desc) = &submission.description {
        desc
    } else {
        return Ok(Redirect::to(uri!(index)));
    };
    let mut file = match File::create(blog_path.join("description.txt")) {
        Ok(f) => f,
        Err(ref e) if e.kind() == ErrorKind::NotFound => {
            // error to user with internal error occured, try again in a few seconds
            return Err(Flash::warning(
                Redirect::to(uri!(new_blog_form)),
                "internal error occured, try again in a few seconds",
            ));
        }
        Err(e) => panic!("unable to create description file for reason: {:?}", e),
    };
    match file.write_all(desc.as_bytes()) {
        Ok(()) => {}
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => {
            // internal error try again
            return Err(Flash::warning(
                Redirect::to(uri!(new_blog_form)),
                "internal error occured, try again in a few seconds",
            ));
        }
        Err(e) => panic!("unable to write to description file for reason: {:?}", e),
    }
    Ok(Redirect::to(uri!(blog_home: &submission.name)))
}

#[get("/blogs/<name>")]
fn blog_home(name: String) -> Option<Content<String>> {
    let blog_path = path_for_blog(&name);
    // this would get all blog posts by this user
    let _dir = match std::fs::read_dir(blog_path.join("posts")) {
        Ok(dir) => dir,
        Err(ref e) if e.kind() == ErrorKind::NotFound => return None,
        Err(e) => panic!(
            "unable to iterate over posts dir for blog: {:?} for reason: {:?}",
            name, e
        ),
    };

    let description = match File::open(blog_path.join("description.txt")) {
        Ok(mut file) => {
            let mut buf = String::new();
            file.read_to_string(&mut buf)
                .expect("unable to *read* from description file");
            buf
        }
        Err(ref e) if e.kind() == ErrorKind::NotFound => "".to_string(),
        Err(e) => panic!(
            "unable to open description for blog: {:?} for reason: {:?}",
            name, e
        ),
    };
    Some(Content(
        ContentType::HTML,
        format!(include!("bloghome.html"), name, description),
    ))
}

#[get("/blogs")]
fn blogs() -> Content<String> {
    let dir = std::fs::read_dir("state/blogs").expect("unable to iterate over blogs dir");
    let mut bloglist = Vec::new();
    for entry in dir {
        //fixme
        let entry = entry.unwrap();
        let path = entry.path();
        let name = name_from_path(&path);
        let description = match File::open(path.join("description.txt")) {
            Ok(mut file) => {
                let mut buf = String::new();
                file.read_to_string(&mut buf)
                    .expect("unable to *read* from description file");
                Some(buf)
            }
            Err(ref e) if e.kind() == ErrorKind::NotFound => None,
            Err(e) => panic!(
                "unable to open description for blog: {:?} for reason: {:?}",
                name, e
            ),
        };
        let meta = BlogMeta { name, description };
        bloglist.push(meta);
    }
    Content(ContentType::HTML, format!(include!("blogs.html"), BlogList { blogs: bloglist } ))
}

fn main() {
    rocket::ignite()
        .mount(
            "/",
            routes![index, new_blog_form, new_blog_accept, blog_home, blogs],
        )
        .launch();
}
