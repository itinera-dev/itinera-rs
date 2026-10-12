@macros @capability-async
Feature: The courier workflow, declared with the macros

  The courier workflow packs a parcel, then dispatches it. Its steps are asynchronous, and
  request the labels and a courier, which its service providers supply: the labels once per
  journey, from a synchronous provider, and a courier for each attempt to dispatch, from an
  asynchronous provider attached to that step, which requests the attempt number and the
  company in the data bag, and reports what it sends. All of it is declared with itinera's
  macros alone. It is asynchronous, so its scenarios run under the asynchronous local executor.

  Scenario: The courier workflow lists its steps, with the provider serving each service
    Given the courier workflow
    When the workflow is listed
    Then the listing is:
      | step     | position | policies | adapter | services                         |
      | pack     | 1        |          |         | labels: labels                   |
      | dispatch | 2        |          |         | courier: courier, labels: labels |

  Scenario: The journey's labels are provided once, and the courier for each attempt
    Given the courier workflow
    And the data bag contains:
      | key     | value      |
      | parcel  | "wardrobe" |
      | company | "swift"    |
    When the workflow runs
    Then the journey succeeded
    And step "dispatch" ended as succeeded after 2 attempts
    And the events are exactly:
      | event                  | step     | attempt | key        | provider | message                 |
      | journey_started        |          |         |            |          |                         |
      | attempt_started        | pack     | 1       |            |          |                         |
      | service_provided       | pack     | 1       | labels     | labels   |                         |
      | step_succeeded         | pack     | 1       |            |          |                         |
      | contribution_committed |          |         | packed     |          |                         |
      | attempt_started        | dispatch | 1       |            |          |                         |
      | journey_info           | dispatch | 1       |            | courier  | sending the swift van   |
      | service_provided       | dispatch | 1       | courier    | courier  |                         |
      | step_failed            | dispatch | 1       |            |          |                         |
      | step_retrying          | dispatch | 1       |            |          |                         |
      | attempt_started        | dispatch | 2       |            |          |                         |
      | journey_info           | dispatch | 2       |            | courier  | sending the swift lorry |
      | service_provided       | dispatch | 2       | courier    | courier  |                         |
      | step_succeeded         | dispatch | 2       |            |          |                         |
      | contribution_committed |          |         | dispatched |          |                         |
      | contribution_committed |          |         | labelled   |          |                         |
      | journey_succeeded      |          |         |            |          |                         |
    And the result's data bag contains:
      | key        | value                          |
      | packed     | "wardrobe for parcel-wardrobe" |
      | dispatched | "wardrobe by swift lorry"      |
      | labelled   | "wardrobe for parcel-wardrobe" |

  Scenario: A provider's missing required data aborts the journey before it is called
    Given the courier workflow
    And the data bag contains:
      | key    | value   |
      | parcel | "books" |
    When the workflow runs
    Then the journey was aborted with the abort reason "required data missing"
    And step "dispatch" ended as aborted after 1 attempts
    And the events include, in order:
      | event           | step     | key     | provider | code                  |
      | attempt_started | dispatch |         |          |                       |
      | journey_aborted | dispatch | company | courier  | required data missing |

  Scenario: An asynchronous provider that fails aborts the journey, since the step could not be built
    Given the courier workflow
    And the data bag contains:
      | key     | value   |
      | parcel  | "books" |
      | company | "none"  |
    When the workflow runs
    Then the journey was aborted with the abort reason "step could not be built"
    And the events include, in order:
      | event                   | step     | attempt | key     | provider | code                    | error      |
      | service_provider_failed | dispatch | 1       | courier | courier  |                         |            |
      | journey_aborted         | dispatch |         |         | courier  | step could not be built | no courier |
    And the result's abort carries the error "no courier"
