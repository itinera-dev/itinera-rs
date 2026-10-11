@macros @capability-async
Feature: The delivery workflow, declared with the macros

  The delivery workflow dispatches a parcel. Its step and its policies are asynchronous, and
  are declared with itinera's macros alone. It is asynchronous, so its scenarios run under the
  asynchronous local executor.

  Scenario: The delivery workflow lists its step, with its policy
    Given the delivery workflow
    When the workflow is listed
    Then the listing is:
      | step     | position | policies | adapter |
      | dispatch | 1        | track    |         |

  Scenario: A parcel is dispatched, tracked and signed for
    Given the delivery workflow
    And the data bag contains:
      | key    | value   |
      | parcel | "books" |
    When the workflow runs
    Then the journey succeeded
    And the journey ID is a UUID v4
    And the events include, in order:
      | event                  | step     |
      | journey_started        |          |
      | attempt_started        | dispatch |
      | step_succeeded         | dispatch |
      | journey_succeeded      |          |
    And the result's data bag contains:
      | key        | value         |
      | dispatched | "books"       |
      | tracking   | "track books" |
      | signed     | true          |

  Scenario: An empty parcel fails the journey, and is not signed for
    Given the delivery workflow
    And the data bag contains:
      | key    | value |
      | parcel | ""    |
    When the workflow runs
    Then the journey failed
    And step "dispatch" ended as failed after 1 attempts
    And the result names the failed step "dispatch" with the code "no-parcel"
    And the result's data bag has no key "signed"
