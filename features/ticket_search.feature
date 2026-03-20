Feature: Ticket Search
  As a user
  I want to search tickets by keyword
  So that I can quickly find relevant work

  Background:
    Given a clean tickets directory

  Scenario: Search matches ticket title
    Given a ticket exists with ID "srch-0001" and title "Export CSV data"
    And a ticket exists with ID "srch-0002" and title "Import JSON data"
    When I run "ticket search csv"
    Then the command should succeed
    And the output should contain "srch-0001"
    And the output should not contain "srch-0002"

  Scenario: Search matches ticket body
    Given a ticket exists with ID "srch-0001" and title "Data pipeline"
    And ticket "srch-0001" has body containing "Supports CSV format for import"
    And a ticket exists with ID "srch-0002" and title "Unrelated ticket"
    When I run "ticket search csv"
    Then the command should succeed
    And the output should contain "srch-0001"
    And the output should not contain "srch-0002"

  Scenario: Search is case-insensitive
    Given a ticket exists with ID "srch-0001" and title "Export CSV Data"
    When I run "ticket search csv"
    Then the command should succeed
    And the output should contain "srch-0001"

  Scenario: Search uppercase query matches lowercase title
    Given a ticket exists with ID "srch-0001" and title "export csv data"
    When I run "ticket search CSV"
    Then the command should succeed
    And the output should contain "srch-0001"

  Scenario: Search excludes closed tickets by default
    Given a ticket exists with ID "srch-0001" and title "CSV export open"
    And a ticket exists with ID "srch-0002" and title "CSV export closed"
    And ticket "srch-0002" has status "closed"
    When I run "ticket search csv"
    Then the command should succeed
    And the output should contain "srch-0001"
    And the output should not contain "srch-0002"

  Scenario: Search with --all includes closed tickets
    Given a ticket exists with ID "srch-0001" and title "CSV export open"
    And a ticket exists with ID "srch-0002" and title "CSV export closed"
    And ticket "srch-0002" has status "closed"
    When I run "ticket search csv --all"
    Then the command should succeed
    And the output should contain "srch-0001"
    And the output should contain "srch-0002"

  Scenario: Search with --status=closed shows only closed matches
    Given a ticket exists with ID "srch-0001" and title "CSV export open"
    And a ticket exists with ID "srch-0002" and title "CSV export closed"
    And ticket "srch-0002" has status "closed"
    When I run "ticket search csv --status=closed"
    Then the command should succeed
    And the output should not contain "srch-0001"
    And the output should contain "srch-0002"

  Scenario: Search with --assignee filter
    Given a ticket exists with ID "srch-0001" and title "CSV for Alice"
    And a ticket exists with ID "srch-0002" and title "CSV for Bob"
    And ticket "srch-0001" has assignee "Alice"
    And ticket "srch-0002" has assignee "Bob"
    When I run "ticket search csv --assignee Alice"
    Then the command should succeed
    And the output should contain "srch-0001"
    And the output should not contain "srch-0002"

  Scenario: Search with --tag filter
    Given a ticket exists with ID "srch-0001" and title "CSV backend"
    And a ticket exists with ID "srch-0002" and title "CSV frontend"
    And ticket "srch-0001" has tags "backend"
    And ticket "srch-0002" has tags "frontend"
    When I run "ticket search csv --tags backend"
    Then the command should succeed
    And the output should contain "srch-0001"
    And the output should not contain "srch-0002"

  Scenario: Search against empty ticket directory shows empty-dir message
    Given a clean tickets directory
    When I run "ticket search csv"
    Then the command should succeed
    And the output should contain "is empty --"

  Scenario: Search with no matches returns empty output
    Given a clean tickets directory
    Given a ticket exists with ID "srch-0001" and title "Something unrelated"
    When I run "ticket search csv"
    Then the command should succeed
    And the output should be empty

  Scenario: Search output format matches ls
    Given a ticket exists with ID "srch-0001" and title "CSV export feature"
    When I run "ticket search csv"
    Then the command should succeed
    And the output should match pattern "srch-0001\s+P2\s+open\s+CSV export feature"
