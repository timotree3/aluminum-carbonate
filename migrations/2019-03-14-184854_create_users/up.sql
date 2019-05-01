CREATE TABLE Users (
    UserID INTEGER PRIMARY KEY ASC NOT NULL,
    Username TEXT NOT NULL UNIQUE,
    Bio TEXT
    -- TODO Profile pictures
);
