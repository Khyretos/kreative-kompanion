-- KG-10: generic project settings (preset, engine, build, test, run, deploy commands, screenshot
-- folder, last result per command) as JSON, for every project type. The repo's computer and folder
-- stay in repo_machine_id / repo_folder (0017).
ALTER TABLE projects ADD COLUMN dev_settings TEXT NOT NULL DEFAULT '';
