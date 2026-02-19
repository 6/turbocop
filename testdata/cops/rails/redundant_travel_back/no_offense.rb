freeze_time
travel_to(Time.zone.now)
Time.now
some_object.travel_back
travel_to(2.days.from_now)

# travel_back outside teardown/after is not flagged
def test_something
  travel_to(1.day.from_now)
  travel_back
end

# travel_back at top level is not flagged
travel_back

# travel_back inside `teardown do...end` block (shoulda-context) is NOT flagged
# because RuboCop only matches `def teardown` and `after do...end`
context "when time is frozen" do
  teardown do
    travel_back
  end
end
