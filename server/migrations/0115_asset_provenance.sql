-- STU-02b: a Studio result sent to Assets keeps where it came from (workflow, prompt and
-- params, models with licences, GPU, when). JSON; NULL for files found by a library scan.
ALTER TABLE asset ADD COLUMN provenance TEXT;
