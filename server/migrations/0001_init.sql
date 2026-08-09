CREATE TABLE articles (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    title        TEXT NOT NULL,
    slug         TEXT NOT NULL UNIQUE,
    markdown     TEXT NOT NULL DEFAULT '',
    status       TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'published')),
    created_at   TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at   TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    published_at TEXT
);
CREATE INDEX idx_articles_status_published_at ON articles (status, published_at DESC);

CREATE TABLE pageviews (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    path       TEXT NOT NULL,
    article_id INTEGER REFERENCES articles (id) ON DELETE SET NULL,
    ip         TEXT NOT NULL,
    user_agent TEXT NOT NULL DEFAULT '',
    referer    TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_pageviews_created_at ON pageviews (created_at);
CREATE INDEX idx_pageviews_ip ON pageviews (ip);
CREATE INDEX idx_pageviews_article_id ON pageviews (article_id);
