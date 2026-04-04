def teardown
  travel_back
  ^^^^^^^^^^^ Rails/RedundantTravelBack: Redundant `travel_back` detected. It is automatically called after each test.
end

after do
  travel_back
  ^^^^^^^^^^^ Rails/RedundantTravelBack: Redundant `travel_back` detected. It is automatically called after each test.
end

after do
  cleanup
  travel_back
  ^^^^^^^^^^^ Rails/RedundantTravelBack: Redundant `travel_back` detected. It is automatically called after each test.
end

module RailsPulse
  module Jobs
    module Cards
      class AverageDurationTest < ActiveSupport::TestCase
        def teardown
          travel_back
          ^^^^^^^^^^^ Rails/RedundantTravelBack: Redundant `travel_back` detected. It is automatically called after each test.
          super
        end
      end
    end
  end
end

RSpec.configure do |config|
  config.after do
    travel_back
    ^^^^^^^^^^^ Rails/RedundantTravelBack: Redundant `travel_back` detected. It is automatically called after each test.
  end
end

RSpec.configure do |config|
  config.after { travel_back }
                 ^^^^^^^^^^^ Rails/RedundantTravelBack: Redundant `travel_back` detected. It is automatically called after each test.
end
