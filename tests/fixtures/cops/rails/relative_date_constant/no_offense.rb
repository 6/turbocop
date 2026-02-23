FOO = "2024-01-01"
BAR = 42
FROZEN = Date.new(2024, 1, 1).freeze
LIMIT = 100
NAME = "constant"
# Time.now, Time.current, Date.today are not relative date methods
TIMESTAMP = Time.now
CURRENT = Time.current
TODAY = Date.today
# Block in constant value — skip per RuboCop
LAZY = -> { 1.week.ago }
