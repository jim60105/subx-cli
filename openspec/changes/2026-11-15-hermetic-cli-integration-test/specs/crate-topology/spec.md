# Spec Delta

## ADDED Requirements

### Requirement: Test Matrix Legs Report Independently

The superproject's test job SHALL run every operating-system matrix leg to completion regardless of another leg's failure, matching the core repository's workflow. A failing leg SHALL NOT cancel its siblings, so each platform's result is reported where the failure occurred.

#### Scenario: A failing platform does not hide its siblings

- **GIVEN** the `test` job matrix over Ubuntu, Windows and macOS
- **WHEN** a test fails on one leg
- **THEN** the other legs SHALL still run to completion and publish their own results