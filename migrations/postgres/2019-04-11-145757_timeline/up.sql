-- Your SQL goes here

CREATE TABLE timeline_definition(
	id SERIAL PRIMARY KEY,
	user_id integer REFERENCES users ON DELETE CASCADE,
	name VARCHAR NOT NULL,
	query VARCHAR NOT NULL
);

CREATE TABLE timeline(
	id SERIAL PRIMARY KEY,
	post_id integer NOT NULL REFERENCES posts ON DELETE CASCADE,
	timeline_id integer NOT NULL REFERENCES timeline_definition ON DELETE CASCADE
);