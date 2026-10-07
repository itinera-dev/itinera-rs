@fixture
Feature: Every Given sentence the runner defines
  Each scenario declares a workflow without running it, so it passes once all of its
  sentences fit the scenario model.

  Scenario: Building a workflow, its data and its journey IDs
    Given a workflow "orders" with the steps:
      | step   |
      | charge |
      | ship   |
    And step "charge" requests input "amount" of type integer
    And step "charge" requests optional input "discount" of type number
    And step "charge" contributes "receipt" = "R-1"
    And step "charge" contributes "items" = [1, 2] and then changes it to [3]
    And step "charge" changes its input "amount" to 7
    And step "charge" emits step_info "charging"
    And step "charge" emits step_warning "slow" with data {"seconds": 3}
    And step "charge" ignores failures of its emit calls and carries on
    And step "charge" attempts:
      | attempt | outcome           | code     | message | details        | contributes        |
      | 1       | retriable failure | declined | later   | {"retry": true} |                    |
      | 2       | error             |          | timeout |                |                    |
      | 3       | skipped           |          |         |                |                    |
      | 4       | success           |          |         |                | {"receipt": "R-2"} |
    And step "charge" allows 3 retries
    And the abnormal termination of step "charge" is retriable
    And step "ship" cannot be built because its constructor fails with "no connection"
    And step "ship" succeeds
    And the data bag contains:
      | key    | value |
      | amount | 42    |
    And the workflow's ID generator returns "order-1"

  Scenario: An empty data bag and the default ID generator
    Given a workflow "orders" with the steps:
      | step   |
      | charge |
    And the data bag is empty
    And the workflow has no ID generator

  Scenario: Input adapters
    Given a workflow "orders" with the steps:
      | step   |
      | charge |
      | refund |
    And the workflow declares the input adapter "pricing" for the steps "charge", "refund"
    And the input adapter "pricing" returns 42 for "amount"
    And the input adapter "pricing" returns nothing for "discount"
    And the input adapter "pricing" fails with "no prices" for "currency"
    And the input adapter "pricing" requests data from the workflow "region" of type string

  Scenario: Policies, hooks and roles
    Given a workflow "orders" with the steps:
      | step   |
      | charge |
    And a step policy "audit" defines the hook "on step success"
    And a step policy "audit" defines the hook "on step failure"
    And a step policy "ledger" defines the hook "on step retry"
    And a workflow policy "notify" defines the hook "on workflow success"
    And step "charge" has the policies "audit", "ledger"
    And the workflow has the policies "notify"
    And the hook "on step success" of policy "audit" requests step data "receipt" of type string
    And the hook "on step success" of policy "audit" requests optional step data "note" of type string
    And the hook "on step success" of policy "audit" changes the step data "receipt" it received to "R-9"
    And the hook "on step success" of policy "audit" requests the step name
    And the hook "on step success" of policy "audit" requests the attempt number
    And the hook "on step success" of policy "audit" requests the journey ID
    And the hook "on step success" of policy "audit" requests data from the workflow "amount" of type integer
    And the hook "on step success" of policy "audit" requests optional data from the workflow "region" of type string
    And the hook "on step success" of policy "audit" contributes "audited" = true
    And the hook "on step success" of policy "audit" counts its calls and contributes the count as "calls"
    And the hook "on step success" of policy "audit" emits journey_info "audited"
    And the hook "on step success" of policy "audit" returns FinishWorkflow
    And the hook "on step failure" of policy "audit" requests the failure reason
    And the hook "on step failure" of policy "audit" requests the failure cause
    And the hook "on step failure" of policy "audit" requests the error
    And the hook "on step failure" of policy "audit" returns FailWorkflow with code "audit failed"
    And the hook "on step retry" of policy "ledger" requests the optional failure reason
    And the hook "on step retry" of policy "ledger" requests the retry cause
    And the hook "on step retry" of policy "ledger" fails with "ledger closed"
    And the workflow provides the role "notifier" with the operation "notify"
    And the hook "on workflow success" of policy "notify" requests the role "notifier" and calls its operation "notify"
    And the operation "notify" of the role "notifier" throws
    And the hook "on workflow success" of policy "notify" throws
    And the policy "ledger" fails when it is built
    And the policy "notify" fails with "no mail server" when it is built

  Scenario: Reporters and dispatchers
    Given a workflow "orders" with the steps:
      | step   |
      | charge |
    And the workflow lists the reporters "audit", "metrics", "fragile"
    And the reporter "audit" throws
    And the reporter "metrics" throws on "attempt_started"
    And the reporter "fragile" fails with "disk full" on "journey_aborted"
    And the executor is given a dispatcher holding the reporter "test"

  Scenario: A dispatcher that ignores added reporters
    Given a workflow "orders" with the steps:
      | step   |
      | charge |
    And the executor is given a dispatcher holding the reporter "test" that ignores added reporters

  Scenario: A dispatcher that fails when a reporter is added
    Given a workflow "orders" with the steps:
      | step   |
      | charge |
    And the executor is given a dispatcher holding the reporter "test" that throws when a reporter is added

  Scenario: A dispatcher that fails when dispatching
    Given a workflow "orders" with the steps:
      | step   |
      | charge |
    And the executor is given a dispatcher holding the reporter "test" that throws when dispatching "step_succeeded"

  Scenario: A dispatcher factory that fails
    Given a workflow "orders" with the steps:
      | step   |
      | charge |
    And the executor is given a dispatcher factory that fails

  Scenario: The default dispatcher
    Given a workflow "orders" with the steps:
      | step   |
      | charge |
    And the executor uses its default dispatcher
