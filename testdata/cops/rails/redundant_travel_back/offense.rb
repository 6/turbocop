def teardown
  travel_back
  ^^^^^^^^^^^ Rails/RedundantTravelBack: Redundant `travel_back` detected. It is automatically called after each test.
end

teardown do
  travel_back
  ^^^^^^^^^^^ Rails/RedundantTravelBack: Redundant `travel_back` detected. It is automatically called after each test.
end

after do
  travel_back
  ^^^^^^^^^^^ Rails/RedundantTravelBack: Redundant `travel_back` detected. It is automatically called after each test.
end
