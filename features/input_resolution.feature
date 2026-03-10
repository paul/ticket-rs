Feature: @ Input Convention for Text Fields
  As a user
  I want to read field values from files or stdin using the @ prefix
  So that I can pass multi-line content and avoid shell quoting issues

  Background:
    Given a clean tickets directory

  # --------------------------------------------------------------------------
  # create --description @file
  # --------------------------------------------------------------------------

  Scenario: Create ticket reads description from file
    Given a file "desc.md" with content "Description from file"
    When I run "ticket create 'My ticket' --description @FILE" with file "desc.md"
    Then the command should succeed
    And the created ticket should contain "Description from file"

  Scenario: Create ticket reads design from file
    Given a file "design.md" with content "Design from file"
    When I run "ticket create 'My ticket' --design @FILE" with file "design.md"
    Then the command should succeed
    And the created ticket should contain "## Design"
    And the created ticket should contain "Design from file"

  Scenario: Create ticket reads acceptance criteria from file
    Given a file "ac.md" with content "Acceptance from file"
    When I run "ticket create 'My ticket' --acceptance @FILE" with file "ac.md"
    Then the command should succeed
    And the created ticket should contain "## Acceptance Criteria"
    And the created ticket should contain "Acceptance from file"

  # --------------------------------------------------------------------------
  # @@ escape — literal @ prefix
  # --------------------------------------------------------------------------

  Scenario: Double @@ produces literal @ prefix in description
    When I run "ticket create 'My ticket' --description @@github"
    Then the command should succeed
    And the created ticket should contain "@github"

  # --------------------------------------------------------------------------
  # update --description @file
  # --------------------------------------------------------------------------

  Scenario: Update ticket reads description from file
    Given a ticket exists with ID "upd-0001" and title "Update target"
    And a file "new_desc.md" with content "Updated description from file"
    When I run "ticket update upd-0001 --description @FILE" with file "new_desc.md"
    Then the command should succeed
    And ticket "upd-0001" should contain "Updated description from file"

  Scenario: Update ticket reads design from file
    Given a ticket exists with ID "upd-0002" and title "Update target"
    And a file "design.md" with content "New design from file"
    When I run "ticket update upd-0002 --design @FILE" with file "design.md"
    Then the command should succeed
    And ticket "upd-0002" should contain "## Design"
    And ticket "upd-0002" should contain "New design from file"

  Scenario: Update ticket reads acceptance from file
    Given a ticket exists with ID "upd-0003" and title "Update target"
    And a file "ac.md" with content "New acceptance from file"
    When I run "ticket update upd-0003 --acceptance @FILE" with file "ac.md"
    Then the command should succeed
    And ticket "upd-0003" should contain "## Acceptance Criteria"
    And ticket "upd-0003" should contain "New acceptance from file"

  # --------------------------------------------------------------------------
  # add-note @file
  # --------------------------------------------------------------------------

  Scenario: Add note reads text from file
    Given a ticket exists with ID "note-0001" and title "Test ticket"
    And a file "note.md" with content "Note from file"
    When I run "ticket note note-0001 @FILE" with file "note.md"
    Then the command should succeed
    And ticket "note-0001" should contain "## Notes"
    And ticket "note-0001" should contain "Note from file"

  # --------------------------------------------------------------------------
  # Stdin input — @- and - syntax
  # --------------------------------------------------------------------------

  Scenario: Create ticket reads description from stdin via @-
    When I run "ticket create 'My ticket' --description @-" with stdin "Design from stdin"
    Then the command should succeed
    And the created ticket should contain "Design from stdin"

  Scenario: Create ticket reads design from stdin via bare -
    When I run "ticket create 'My ticket' --design -" with stdin "Design piped in"
    Then the command should succeed
    And the created ticket should contain "## Design"
    And the created ticket should contain "Design piped in"

  Scenario: Stdin input strips exactly one trailing newline
    When I run "ticket create 'My ticket' --description @-" with stdin "Content\n"
    Then the command should succeed
    And the created ticket should contain "Content"

  Scenario: Stdin input preserves multiple trailing newlines except the last
    # Input "Content\n\n" has two trailing newlines; only one is stripped, so
    # the description stored in the ticket is "Content\n" (a blank line after
    # the word). We verify this by checking that a blank line follows "Content".
    When I run "ticket create 'My ticket' --description @-" with stdin "Content\n\n"
    Then the command should succeed
    And the created ticket body preserves trailing blank line after "Content"

  Scenario: add-note reads from stdin explicitly via @-
    Given a ticket exists with ID "note-0001" and title "Test ticket"
    When I run "ticket note note-0001 @-" with stdin "Note from explicit stdin"
    Then the command should succeed
    And ticket "note-0001" should contain "Note from explicit stdin"

  Scenario: add-note reads from stdin fallback when no argument given
    Given a ticket exists with ID "note-0001" and title "Test ticket"
    When I run "ticket note note-0001" with stdin "Note from stdin fallback"
    Then the command should succeed
    And ticket "note-0001" should contain "Note from stdin fallback"

  # --------------------------------------------------------------------------
  # Error cases
  # --------------------------------------------------------------------------

  Scenario: File not found produces clear error
    When I run "ticket create 'My ticket' --description @/nonexistent/path/file.md"
    Then the command should fail
    And the output should contain "cannot read"
    And the output should contain "/nonexistent/path/file.md"

  Scenario: Multiple stdin fields produce error
    When I run "ticket create 'My ticket' --description - --design -" with no stdin
    Then the command should fail
    And the output should contain "only one field may read from stdin at a time"
