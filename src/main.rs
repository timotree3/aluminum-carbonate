#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
#[macro_use] extern crate rocket_contrib;
#[macro_use] extern crate diesel;

mod schema;

use crate::schema::Posts::dsl::*;
use crate::schema::Users::dsl::*;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use rocket::Rocket;
use rocket_contrib::templates::Template;
use std::error::Error;
use render::Content;

#[database("sqlite_database")]
struct DbConn(SqliteConnection);

#[get("/blogs/<user>/<title>")]
fn blog_post(user: String, title: String, conn: DbConn) -> Result<Content, Box<Error>> {
    let user_ids = Users.select(UserID).filter(Username.eq(user)).load(&conn)?;
    assert!(user_ids.len() <= 1);
    match user_ids.into_iter().next() {
        Some(user_id) => {
            let content_vec = Posts.select(Content).filter(AuthorID.eq(user_id).and(Title.eq(title))).load(&conn)?;
            assert!(content_vec.len() <= 1);
            match content_vec.get(0) {
                Some(body) => {
                    Ok(render::blog_post(user, title, body))
                },
                None => {
                    // page not found
                    panic!()
                }
            }
        }
        None => {
            // page not found
            panic!()
        }
    }
}

#[get("/blogs/<user>")]
fn blog(user: String, conn: DbConn) -> Result<Content, Box<Error>> {
    let user_ids = Users.select(UserID).filter(Username.eq(user)).load(&conn)?;
    assert!(user_ids.len() <= 1);
    match user_ids.into_iter().next() {
        Some(user_id) => {
            let titles = Posts.select(Title).filter(AuthorID.eq(user_id)).load(&conn)?;
            Ok(render::blog_home(user, titles))
        }
        None => {
            // page not found
            panic!()
        }
    }
}

#[get("/blogs")]
fn blogs (conn: DbConn) -> Result<Content, Box<Error>> {
    let blog_list = Users.select(Username).load(&conn)?;
    Ok(render::blog_dir(blog_list))
}

#[get("/")]
fn index() -> Content {
    render::home_page()
}

pub fn rocket() -> Rocket {
    rocket::ignite()
        .mount("/", routes![blog_post, blog, /*blogs,*/ index])
        .attach(DbConn::fairing())
        .attach(Template::fairing())
}

mod render {
    pub fn home_page() -> Content {
        Content
        //Template::render("index", &json!({}))
    }
    pub fn login() -> Content { Content }
    pub fn blog_dir(/*what belongs here?*/) -> Content {
        Content
        // Template::render("blogs", &template_context)
    }
    pub fn blog_home(username: String, titles: Vec<String>) -> Content {
        Content
        // Template::render("blog", &blog)
    }
    pub fn blog_post() -> Content {
        Content
    }
    pub fn edit_post(/* what belongs here*/) -> Content { Content }

    pub struct Content;

    impl<'a> rocket::response::Responder<'a> for Content {
        fn respond_to(self, _req: &rocket::Request<'_>) -> Result<rocket::Response<'a>, rocket::http::Status> {
            unimplemented!()
        }
    }
}

fn main() {
    rocket().launch();
}
