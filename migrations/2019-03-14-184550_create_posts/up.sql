CREATE TABLE Posts (
    PostID INTEGER PRIMARY KEY ASC NOT NULL,
    AuthorID INTEGER NOT NULL,
    Title TEXT NOT NULL UNIQUE,
    Body TEXT NOT NULL -- TODO: consider more structured markup format
);
