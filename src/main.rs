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

#[derive(Debug)]
enum DirError {
    AlreadyExists,
    DeletedDuring,
    CreationFailure(std::io::Error),
    ClosureFailure(std::io::Error),
}

fn create_dir_with<A, F, T>(path: A, code: F) -> Result<T, DirError>
where
    A: AsRef<Path>,
    F: FnOnce() -> Result<T, std::io::Error>,
{
    match std::fs::create_dir(path) {
        Ok(()) => {}
        Err(ref e) if e.kind() == ErrorKind::AlreadyExists => return Err(DirError::AlreadyExists),
        Err(e) => return Err(DirError::CreationFailure(e)),
    }
    match code() {
        Ok(v) => Ok(v),
        Err(ref e) if e.kind() == ErrorKind::NotFound => Err(DirError::DeletedDuring),
        Err(e) => Err(DirError::ClosureFailure(e)),
    }
}

fn base64_config() -> base64::Config {
    base64::Config::new(base64::CharacterSet::UrlSafe, false)
}

fn path_for_blog(name: &str) -> PathBuf {
    let mut path = "state/blogs/".to_string();
    base64::encode_config_buf(name, base64_config(), &mut path);
    dbg!(path.into())
}

fn name_from_path<A: AsRef<Path>>(path: A) -> String {
    let encoded = path
        .as_ref()
        .file_name()
        .expect("name_from_path called with invalid blog path")
        .to_str()
        .expect("base64 encoded blog name isn't valid utf8");
    String::from_utf8(
        base64::decode_config(encoded, base64_config()).expect("blog name isn't valid base64"),
    )
    .expect("blog name isn't valid utf8")
}

#[derive(FromForm)]
struct BlogMeta {
    name: String,
    description: Option<String>,
}

impl Display for BlogMeta {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "{}: {}",
            &self.name,
            self.description.as_ref().map(|x| &**x).unwrap_or("")
        )
    }
}

struct BlogList {
    blogs: Vec<BlogMeta>,
}

impl Display for BlogList {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "<ul>")?;
        for meta in &self.blogs {
            write!(
                f,
                "<li><a href=\"{}/\">{}</a></li>",
                uri!(blog_home: &meta.name),
                meta
            )?;
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
    let result = create_dir_with(&blog_path, || {
        std::fs::create_dir(blog_path.join("posts"))?;
        if let Some(desc) = &submission.description {
            let mut file = File::create(blog_path.join("description.txt"))?;
            file.write_all(desc.as_bytes())?;
        }
        Ok(Redirect::to(uri!(blog_home: &submission.name)))
    });
    result.map_err(|e| match e {
        DirError::AlreadyExists => {
            // name taken
            Flash::error(Redirect::to(uri!(new_blog_form)), "name taken")
        }
        DirError::DeletedDuring => {
            // error to user with internal error occured, try again in a few seconds
            Flash::warning(
                Redirect::to(uri!(new_blog_form)),
                "internal error occured, try again in a few seconds",
            )
        }
        e => panic!("unable to create new blog for reason: {:?}", e),
    })
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
    Content(
        ContentType::HTML,
        format!(include!("blogs.html"), BlogList { blogs: bloglist }),
    )
}

#[derive(FromForm)]
struct DraftData {
    title: String,
    body: String,
}

#[get("/blogs/<name>/create")]
fn create_post(name: String) -> Option<Content<&'static str>> {
    let path = path_for_blog(&name);
    if path.exists() {
        Some(Content(ContentType::HTML, include_str!("createpost.html")))
    } else {
        None
    }
}

fn post_suffix(title: &str) -> PathBuf {
    let b64_encoded = base64::encode_config(title, base64_config());
    Path::new("posts").join(b64_encoded)
}

#[post("/blogs/<name>/create", data = "<submission>")]
fn new_post_accept(
    name: String,
    submission: rocket::request::Form<DraftData>,
) -> Result<Redirect, Flash<Redirect>> {
    // create new folder with base64 encoded title
    let post_path = path_for_blog(&name).join(post_suffix(&submission.title));
    create_dir_with(&post_path, || {
        // create file in folder called body.txt
        let mut f = File::create(post_path.join("body.txt"))?;
        // write content into body.txt
        f.write_all(submission.body.as_bytes())?;
        // return redirect to post link
        Ok(Redirect::to(uri!(index)))
    })
    .map_err(|e| match e {
        DirError::AlreadyExists => Flash::warning(
            Redirect::to(uri!(create_post: name)),
            "a post with that title already exists",
        ),
        DirError::DeletedDuring => Flash::error(
            Redirect::to(uri!(blogs)),
            "that blog or post has since been deleted",
        ),
        DirError::CreationFailure(ref e) if e.kind() == ErrorKind::NotFound => Flash::error(
            Redirect::to(uri!(blogs)),
            "that blog doesn't exist or has been deleted",
        ),
        e => panic!(
            "failed to create files for post. blog: {:?}, post_title: {:?}, error: {:?}",
            name, submission.title, e
        ),
    })
}

#[get("/blogs/<name>/p/<title>")]
fn post(name: String, title: String) -> Option<Content<String>> {
    let blog_path = path_for_blog(name: &str);
    let result = || -> Result<(String, String), io::Error> {
        let f = File::open(blog_path.join(post_suffix(&title).join("body.txt")))?;
        let mut body = String::new();
        f.read_to_string(&mut body)?;
        f = File::open(blog_path.join("description.txt"))?;
        let mut description = String::new();
        f.read_to_string(&mut description)?;
        Ok((description, body))
    }()
    match result {
        
        Err(ref e) if e.kind() = ErrorKind::NotFound => {
            return None();
        }
        Err(e) {
            panic!("unexpected error when displaying post: {:?}", e);
        }
    }


    Ok(Content(ContentType::HTML, format!(include!("post.html"), name, description, title, body)))
}

fn main() {
    rocket::ignite()
        .mount(
            "/",
            routes![
                index,
                new_blog_form,
                new_blog_accept,
                blog_home,
                blogs,
                create_post,
                new_post_accept
            ],
        )
        .launch();
}
