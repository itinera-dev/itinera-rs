@macros @capability-sync
Feature: The payment workflow, declared with the macros

  The payment workflow charges an order's price, less its discount, then ships it. Its steps,
  its policies, the role its receipts use, its input adapter and its ID generator are declared
  with itinera's macros alone. It is synchronous, so its scenarios run under the local executor.

  Scenario: The payment workflow lists its steps, with their policies and input adapter
    Given the payment workflow
    When the workflow is listed
    Then the listing is:
      | step   | position | policies | adapter |
      | charge | 1        | audit    | pricing |
      | ship   | 2        |          |         |

  Scenario: A priced order is charged, audited, shipped and given a receipt
    Given the payment workflow
    And the data bag contains:
      | key      | value  |
      | order    | "1234" |
      | price    | 40     |
      | discount | 5      |
    When the workflow runs
    Then the journey succeeded
    And the journey ID is "corner-1234"
    And step "charge" ended as succeeded after 1 attempts
    And step "ship" ended as succeeded after 1 attempts
    And the result's data bag contains:
      | key     | value                                    |
      | charged | 35                                       |
      | audited | "charged 35"                             |
      | shipped | true                                     |
      | receipt | "corner-1234 paid 35 at the corner shop" |

  Scenario: A declined charge is retried once, then fails the journey
    Given the payment workflow
    And the data bag contains:
      | key   | value  |
      | order | "1235" |
      | price | 0      |
    When the workflow runs
    Then the journey failed
    And step "charge" ended as failed after 2 attempts
    And the result names the failed step "charge" with the code "declined"
    And the result's data bag contains:
      | key     | value      |
      | receipt | "declined" |
    And the result's data bag has no key "shipped"

  Scenario: An order without a price is aborted, since nothing supplies the amount to charge
    Given the payment workflow
    And the data bag contains:
      | key   | value  |
      | order | "1236" |
    When the workflow runs
    Then the journey was aborted with the abort reason "required data missing"
    And step "charge" ended as aborted
