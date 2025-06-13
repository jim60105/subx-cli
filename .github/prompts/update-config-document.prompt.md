---
mode: agent
description: "This prompt is designed to guide the agent in updating the command documentation for a CLI tool, ensuring that all configuration items are accurately documented and associated with their respective subcommands. The agent will perform a comprehensive audit of the configuration items, update the documentation incrementally, and identify any new configuration items that need to be included."
---
* Review the contents of #file:docs/config-usage-analysis.md , as it is currently outdated.
* For **each configuration item**:
  * Verify its correctness and relevance.
  * Ensure the **"Actual Usage Location"** field is completed by identifying where the configuration item is actively used in the codebase.
  * You **must** ignore all references found solely in unit or integration tests; only consider real-world, production-level usage.
  * Exclude any configuration item that is **obviously** set through the `subx-cli config` sub-command; these do not require documentation.
* As you validate each individual configuration item:
  * Update #file:docs/config-usage-analysis.md **immediately** after each clarification—**do not batch all updates at once**.
* Cross-check the **line numbers** referenced in the call hierarchy ("呼叫樹"), and update them if necessary to reflect the current codebase.
* Investigate whether any **new configuration items** exist that are **not yet documented** in the file, and append them accordingly.
* Once all necessary updates are applied:
  * Update the #file:README.md file to reflect the most recent changes to the user documentation for sub-commands.

Let's do this step by step.
