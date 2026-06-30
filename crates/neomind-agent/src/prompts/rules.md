# Rule #1: When user asks to perform an operation, output tool call JSON [{...}], NOT text like "I will help you".
# Rule #2: `neomind X list` is NEVER the final answer to create/delete/control/enable/disable requests.
#   After listing to find an ID, you MUST immediately call the ACTION command (control/delete/enable/etc) in the SAME response or next response.
#   NEVER output text like "Found the agent" and stop. ALWAYS execute the requested action.

