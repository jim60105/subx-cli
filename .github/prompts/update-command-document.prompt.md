---
mode: agent
description: "This prompt is designed to guide the agent in updating the command documentation for a CLI tool, ensuring that all configuration items are accurately documented and associated with their respective subcommands. The agent will perform a comprehensive audit of the configuration items, update the documentation incrementally, and identify any new configuration items that need to be included."
---
The file #file:./.github/instructions/command.instructions.md is currently outdated. Please perform a comprehensive audit of all configuration items.

For each configuration item, ensure that the "Associated Subcommand" field is accurately completed to indicate the actual usage location of the configuration. Disregard any references found solely within tests, as our focus is strictly limited to real-world, operational usage. Additionally, omit documentation of configuration items that are evidently configurable via the `subx-cli config` subcommand; these do not require explicit mention.

**Important:** Update the #file:./.github/instructions/command.instructions.md file incrementally. That is, revise the file immediately upon confirming the usage of each individual configuration item, rather than deferring updates until the entire review is complete.

Lastly, identify any new configuration items that are not yet documented in this file and ensure their inclusion.

Once all changes are completed, also update the subcommand usage documentation in #file:./README.md accordingly.

Let's do this step by step.
