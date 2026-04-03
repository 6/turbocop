Time.now
Date.iso8601('2016-06-29')
DateTime.iso8601('2016-06-29', Date::ENGLAND)
::DateTime.iso8601('2016-06-29', ::Date::ITALY)
Icalendar::Values::DateTime.new(start_at)
x = 1

# Bare helper-style to_datetime call with arguments should not be flagged
to_datetime(row["created_at"])
to_datetime("2024-01-01")

# Receiver calls with arguments are not DateTime coercions for this cop
scope.to_datetime(applicable_to_datetime)
object.to_datetime(date)
