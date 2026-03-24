Time.now
Date.iso8601('2016-06-29')
DateTime.iso8601('2016-06-29', Date::ENGLAND)
::DateTime.iso8601('2016-06-29', ::Date::ITALY)
Icalendar::Values::DateTime.new(start_at)
x = 1

# Bare to_datetime call (local method, no receiver) should not be flagged
to_datetime(row["created_at"])
to_datetime("2024-01-01")
