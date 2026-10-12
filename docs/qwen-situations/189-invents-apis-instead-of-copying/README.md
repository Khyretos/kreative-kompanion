# Lesson 189: the Coder invents APIs instead of copying the template function it was given

Situation: job `gp-web-model` (M6-11), second run (job.json): "copy `plan_with_model` of planner_web.rs (in the context) but call `gameplan::generate`". planner_web.rs was in the context.
Wrong output (OVMS, 4 attempts, 5 calls, never compiled): wrong-gameplan_web.rs.txt calls `llm::chat(system, user, None)` and a non-existent `gameplan::parse_response`, ignoring the `chat_json` / `chat_with_tools_full` closure of the template. Repair rounds only varied the invented names.
Corrected: Claude copied the template with 3 changes (schema from gameplan, generate from gameplan, error text): corrected-plan_with_model.rs.txt (22 lines); it compiles.
Lesson: when a function must mirror another, put the template's body into the job prompt as the text to adapt ("change exactly these 3 things"), not as a file in the context; a 9B model that has to find and adapt a body invents instead. Rerun after model refinement: run job.json and check it copies the closure.
