travel_back
^^^^^^^^^^^ Rails/RedundantTravelBack: Redundant `travel_back` detected. It is automatically called after each test.

def test_something
  travel_to(1.day.from_now)
  assert_equal Date.tomorrow, Date.current
  travel_back
  ^^^^^^^^^^^ Rails/RedundantTravelBack: Redundant `travel_back` detected. It is automatically called after each test.
end