@fixture
Feature: A Scenario Outline, whose rows share the scenario's name
  Scenario Outline: The same check for several steps
    Given a sentence that no catalogue has for "<step>"

    Examples:
      | step   |
      | charge |
      | refund |
      | ship   |

    Examples:
      | step   |
      | cancel |
