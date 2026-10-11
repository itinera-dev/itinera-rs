@macros @capability-sync
Feature: The warehouse workflow, declared with the macros

  The warehouse workflow reserves an order's goods, charges it, then ships it. Its steps request
  the stock and the payments, which its service providers supply: the stock once per journey,
  the payments of every step from one provider, and fresh payments for each attempt to charge
  from a provider attached to that step, which requests the step's name, the attempt number and
  the gateway in the data bag, and reports which gateway it chooses. All of it is declared with
  itinera's macros alone. It is synchronous, so its scenarios run under the local executor.

  Scenario: The warehouse workflow lists its steps, with the provider serving each service
    Given the warehouse workflow
    When the workflow is listed
    Then the listing is:
      | step    | position | policies | adapter | services                               |
      | reserve | 1        |          |         | stock: stock                           |
      | charge  | 2        |          |         | payments: fresh payments, stock: stock |
      | ship    | 3        |          |         | payments: payments, stock: stock       |

  Scenario: The journey's stock is provided once, and the attached provider serves only its step
    Given the warehouse workflow
    And the data bag contains:
      | key     | value  |
      | order   | "1"    |
      | gateway | "main" |
    When the workflow runs
    Then the journey succeeded
    And the journey ID is "order-1"
    And the events are exactly:
      | event                  | step    | attempt | key      | provider       | message                   |
      | journey_started        |         |         |          |                |                           |
      | attempt_started        | reserve | 1       |          |                |                           |
      | service_provided       | reserve | 1       | stock    | stock          |                           |
      | step_succeeded         | reserve | 1       |          |                |                           |
      | contribution_committed |         |         | reserved |                |                           |
      | attempt_started        | charge  | 1       |          |                |                           |
      | journey_info           | charge  | 1       |          | fresh payments | choosing the main gateway |
      | service_provided       | charge  | 1       | payments | fresh payments |                           |
      | step_succeeded         | charge  | 1       |          |                |                           |
      | contribution_committed |         |         | charged  |                |                           |
      | attempt_started        | ship    | 1       |          |                |                           |
      | service_provided       | ship    | 1       | payments | payments       |                           |
      | step_succeeded         | ship    | 1       |          |                |                           |
      | contribution_committed |         |         | shipped  |                |                           |
      | journey_succeeded      |         |         |          |                |                           |
    And the result's data bag contains:
      | key      | value                                                      |
      | reserved | "stock of order-1"                                         |
      | charged  | "main gateway for charge, attempt 1 with stock of order-1" |
      | shipped  | "payments with stock of order-1"                           |

  Scenario: A retried charge is given fresh payments, and the same stock, on each attempt
    Given the warehouse workflow
    And the data bag contains:
      | key     | value   |
      | order   | "2"     |
      | gateway | "flaky" |
    When the workflow runs
    Then the journey succeeded
    And step "charge" ended as succeeded after 2 attempts
    And the events are exactly:
      | event                  | step    | attempt | key      | provider       | message                    |
      | journey_started        |         |         |          |                |                            |
      | attempt_started        | reserve | 1       |          |                |                            |
      | service_provided       | reserve | 1       | stock    | stock          |                            |
      | step_succeeded         | reserve | 1       |          |                |                            |
      | contribution_committed |         |         | reserved |                |                            |
      | attempt_started        | charge  | 1       |          |                |                            |
      | journey_info           | charge  | 1       |          | fresh payments | choosing the flaky gateway |
      | service_provided       | charge  | 1       | payments | fresh payments |                            |
      | step_failed            | charge  | 1       |          |                |                            |
      | step_retrying          | charge  | 1       |          |                |                            |
      | attempt_started        | charge  | 2       |          |                |                            |
      | journey_info           | charge  | 2       |          | fresh payments | choosing the flaky gateway |
      | service_provided       | charge  | 2       | payments | fresh payments |                            |
      | step_succeeded         | charge  | 2       |          |                |                            |
      | contribution_committed |         |         | charged  |                |                            |
      | attempt_started        | ship    | 1       |          |                |                            |
      | service_provided       | ship    | 1       | payments | payments       |                            |
      | step_succeeded         | ship    | 1       |          |                |                            |
      | contribution_committed |         |         | shipped  |                |                            |
      | journey_succeeded      |         |         |          |                |                            |
    And the result's data bag contains:
      | key     | value                                                       |
      | charged | "flaky gateway for charge, attempt 2 with stock of order-2" |

  Scenario: A provider whose optional data is absent emits optional_input_absent naming it
    Given the warehouse workflow
    And the data bag contains:
      | key   | value |
      | order | "3"   |
    When the workflow runs
    Then the journey succeeded
    And the events include, in order:
      | event                 | step   | attempt | key      | provider       | message                   |
      | attempt_started       | charge | 1       |          |                |                           |
      | optional_input_absent | charge | 1       | gateway  | fresh payments |                           |
      | journey_info          | charge | 1       |          | fresh payments | choosing the main gateway |
      | service_provided      | charge | 1       | payments | fresh payments |                           |
    And the result's data bag contains:
      | key     | value                                                      |
      | charged | "main gateway for charge, attempt 1 with stock of order-3" |

  Scenario: A provider that fails aborts the journey, since the step could not be built
    Given the warehouse workflow
    And the data bag contains:
      | key     | value  |
      | order   | "4"    |
      | gateway | "down" |
    When the workflow runs
    Then the journey was aborted with the abort reason "step could not be built"
    And step "charge" ended as aborted after 1 attempts
    And the events include, in order:
      | event                   | step   | attempt | key      | provider       | code                    | error        |
      | attempt_started         | charge | 1       |          |                |                         |              |
      | service_provider_failed | charge | 1       | payments | fresh payments |                         |              |
      | journey_aborted         | charge |         |          | fresh payments | step could not be built | gateway down |
    And the result's abort carries the error "gateway down"
