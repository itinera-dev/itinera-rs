@fixture
Feature: Admitting and listing a workflow
  The workflow is built from the scenario model, which checks its declaration and runs
  nothing.

  Scenario: A workflow is listed with its policies and adapters
    Given a workflow "orders" with the steps:
      | step   |
      | charge |
      | ship   |
    And a step policy "audit" defines the hook "on step success"
    And a step policy "alarm" defines the hook "on step failure"
    And step "charge" has the policies "audit", "alarm"
    And the workflow declares the input adapter "pricing" for the steps "ship"
    When the workflow is listed
    Then the listing is:
      | step   | position | policies     | adapter |
      | charge | 1        | audit, alarm |         |
      | ship   | 2        |              | pricing |
    And no step ran

  Scenario: Every violation is refused together
    Given a workflow "orders" with the steps:
      | step   |
      | charge |
      | ship   |
      | ship   |
    And a step policy "audit" defines the hook "on step success"
    And a step policy "metrics" defines the hook "on step success"
    And step "charge" has the policies "audit", "metrics"
    And a workflow policy "notify" defines the hook "on workflow success"
    And a workflow policy "archive" defines the hook "on workflow success"
    And the workflow has the policies "notify", "archive"
    And the workflow declares the input adapter "pricing" for the steps "charge"
    And the workflow declares the input adapter "discounts" for the steps "charge", "refund"
    When the workflow is admitted
    Then admission is refused with the violations:
      | violation                      |
      | input adapter for unknown step |
      | hook defined twice             |
      | duplicate step name            |
      | step adapted twice             |
      | hook defined twice             |
    And no step ran

  @mismatch
  Scenario: A listing that differs from the table
    Given a workflow "orders" with the steps:
      | step   |
      | charge |
    When the workflow is listed
    Then the listing is:
      | step | position | policies | adapter |
      | ship | 1        |          |         |

  @mismatch
  Scenario: An admitted workflow said to be refused
    Given a workflow "orders" with the steps:
      | step   |
      | charge |
    When the workflow is admitted
    Then admission is refused with the violations:
      | violation           |
      | duplicate step name |
