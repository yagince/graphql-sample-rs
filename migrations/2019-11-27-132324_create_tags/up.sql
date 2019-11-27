CREATE TABLE tags (
  id   SERIAL  PRIMARY KEY,
  user_id INT  NOT NULL references users(id),
  name VARCHAR NOT NULL
);
