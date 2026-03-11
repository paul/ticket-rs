Feature: Configuration via .tickets.toml and environment variables
  As a user
  I want to configure ticket_prefix and ticket_dir in .tickets.toml or via env vars
  So that I can customise ticket IDs and store location without changing my workflow

  Background:
    Given a clean tickets directory

  # ---------------------------------------------------------------------------
  # ticket_prefix
  # ---------------------------------------------------------------------------

  Scenario: TICKET_PREFIX env var overrides derived prefix
    When I run "ticket create 'My ticket'" with TICKET_PREFIX set to "xyz"
    Then the command should succeed
    And the output should match the prefix "xyz"

  Scenario: ticket_prefix in .tickets.toml overrides derived prefix
    Given a .tickets.toml file with content:
      """
      ticket_prefix = "cfg"
      """
    When I run "ticket create 'Config ticket'"
    Then the command should succeed
    And the output should match the prefix "cfg"

  Scenario: TICKET_PREFIX env var takes priority over .tickets.toml prefix
    Given a .tickets.toml file with content:
      """
      ticket_prefix = "file"
      """
    When I run "ticket create 'Override ticket'" with TICKET_PREFIX set to "env"
    Then the command should succeed
    And the output should match the prefix "env"

  # ---------------------------------------------------------------------------
  # ticket_dir
  # ---------------------------------------------------------------------------

  Scenario: TICKET_DIR env var overrides the default .tickets location
    Given a separate tickets directory exists at "alt-tickets" with ticket "alt-0001" titled "Alt ticket"
    When I run "ticket ls" with TICKET_DIR set to "alt-tickets"
    Then the command should succeed
    And the output should contain "alt-0001"

  Scenario: ticket_dir in .tickets.toml overrides the default .tickets location
    Given a separate tickets directory exists at "alt-tickets" with ticket "cfg-0001" titled "Config dir ticket"
    Given a .tickets.toml file with content:
      """
      ticket_dir = "alt-tickets"
      """
    When I run "ticket ls"
    Then the command should succeed
    And the output should contain "cfg-0001"

  Scenario: TICKET_DIR env var takes priority over .tickets.toml ticket_dir
    Given a separate tickets directory exists at "env-tickets" with ticket "env-0001" titled "Env ticket"
    Given a separate tickets directory exists at "file-tickets" with ticket "file-0001" titled "File ticket"
    Given a .tickets.toml file with content:
      """
      ticket_dir = "file-tickets"
      """
    When I run "ticket ls" with TICKET_DIR set to "env-tickets"
    Then the command should succeed
    And the output should contain "env-0001"
    And the output should not contain "file-0001"

  Scenario: Legacy TICKETS_DIR env var still works as an alias for TICKET_DIR
    Given a separate tickets directory exists at "legacy-tickets" with ticket "leg-0001" titled "Legacy ticket"
    When I run "ticket ls" with TICKETS_DIR set to "legacy-tickets"
    Then the command should succeed
    And the output should contain "leg-0001"

  # ---------------------------------------------------------------------------
  # .tickets.toml discovery
  # ---------------------------------------------------------------------------

  Scenario: .tickets.toml is found by walking up from a subdirectory
    Given a .tickets.toml file with content:
      """
      ticket_prefix = "walk"
      """
    And I am in subdirectory "deep/sub/dir"
    When I run "ticket create 'Deep ticket'"
    Then the command should succeed
    And the output should match the prefix "walk"
