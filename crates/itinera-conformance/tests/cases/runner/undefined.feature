@fixture
Feature: Scenarios using a sentence the runner does not define

  @selected
  Scenario: A selected scenario
    Given a sentence that no catalogue has

  Scenario: A scenario left out
    Given a sentence that no catalogue has

  @selected @capability-async
  Scenario: A selected scenario that needs the async capability
    Given a sentence that no catalogue has

  @selected @capability-sync
  Scenario: A selected scenario that needs the sync capability
    Given a sentence that no catalogue has
