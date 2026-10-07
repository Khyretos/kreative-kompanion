-- CHAT-03b: collections per project (a project chat searches them first) and a hash per
-- document, so a re-import from Open WebUI replaces only the files that changed.
CREATE TABLE knowledge_project (
    collection_id TEXT NOT NULL REFERENCES knowledge_collection(id) ON DELETE CASCADE,
    project_id    TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    PRIMARY KEY (collection_id, project_id)
);
CREATE INDEX knowledge_project_by_project ON knowledge_project(project_id);
ALTER TABLE knowledge_doc ADD COLUMN hash TEXT NOT NULL DEFAULT '';
