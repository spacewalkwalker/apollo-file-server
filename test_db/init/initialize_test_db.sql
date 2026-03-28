CREATE TABLE chart_sets (
  chart_set_id integer PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
  title text,
  artist text
);

CREATE TABLE charts (
  chart_id integer PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
  chart_set_id integer REFERENCES chart_sets (chart_set_id),
  designator text,
  file_id text NOT NULL,
  metadata jsonb
);

CREATE TABLE chart_set_aux_files (
  chart_set_id integer REFERENCES chart_sets (chart_set_id),
  label text NOT NULL,
  filename text,
  file_id text NOT NULL
);

CREATE TABLE api_keys (
  api_key text
);

INSERT INTO api_keys (api_key) VALUES ('test-key-1234');
