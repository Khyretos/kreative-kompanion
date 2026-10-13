# Lesson 233: prose pseudo-code is copied as code (2026-10-13, PROJ-OVERVIEW w4 w27)

My w4 prompt (mock getOverview) wrote conditions as prose: "grants: when m and folder: (...), else []". The Coder pasted `when(m && folder) : ... else [],` into TypeScript three times. The same prompt named a local `grants` that shadows the module's `grants`, which broke the corrected version too (my fault). In w27 the Coder turned my check message "the overview form submit is missing" into a code comment and wrote an `overview-save` click action with an invented `api.setOverview` in the actions map instead of the submit listener.

Rule (lesson 85 again): conditions as code (`m && folder ? ... : []`), never a local that shadows a module name, and check messages say what to do, not what is missing.
