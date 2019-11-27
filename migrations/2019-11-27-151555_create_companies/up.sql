CREATE TABLE companies (
  id   SERIAL  PRIMARY KEY,
  name VARCHAR NOT NULL
);

CREATE TABLE employments (
  id         SERIAL PRIMARY KEY,
  user_id    INT    NOT NULL references users(id),
  company_id INT    NOT NULL references companies(id)
);
