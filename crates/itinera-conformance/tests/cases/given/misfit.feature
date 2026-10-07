@misfit
Feature: A sentence that does not fit the scenario declared so far

  Scenario: A step the workflow does not have
    Given a workflow "orders" with the steps:
      | step   |
      | charge |
    And step "ship" succeeds
