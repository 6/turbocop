DateTime.now
^^^^^^^^^^^^ Style/DateTime: Prefer `Time` over `DateTime`.

::DateTime.now
^^^^^^^^^^^^^^ Style/DateTime: Prefer `Time` over `DateTime`.

DateTime.iso8601('2016-06-29')
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DateTime: Prefer `Time` over `DateTime`.

DateTime.new(2024, 1, 1)
^^^^^^^^^^^^^^^^^^^^^^^^ Style/DateTime: Prefer `Time` over `DateTime`.

DateTime.civil(2024, 1, 1, 12, 0, 0)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DateTime: Prefer `Time` over `DateTime`.

DateTime.parse('2024-01-01')
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DateTime: Prefer `Time` over `DateTime`.

thing.to_datetime
^^^^^^^^^^^^^^^^^ Style/DateTime: Do not use `#to_datetime`.

DateTime&.now
^^^^^^^^^^^^^ Style/DateTime: Prefer `Time` over `DateTime`.

to_datetime <=> other
^ Style/DateTime: Do not use `#to_datetime`.

result = to_datetime.since(seconds)
         ^^^^^^^^^^^ Style/DateTime: Do not use `#to_datetime`.

to_datetime <=> other
^ Style/DateTime: Do not use `#to_datetime`.

period; utc; time; to_datetime; to_time
                   ^^^^^^^^^^^ Style/DateTime: Do not use `#to_datetime`.

DateTime.new(*datetime_params, 0, Date::GREGORIAN) :
^ Style/DateTime: Prefer `Time` over `DateTime`.

assert_equal_with_offset(DateTime.new(1582, 10, 14, 0, 0, 0, 0, Date::GREGORIAN), dt)
                         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DateTime: Prefer `Time` over `DateTime`.

for_test(DateTime.new(1582,10,4,0,0,0,0,Date::ITALY)) do |t|
         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DateTime: Prefer `Time` over `DateTime`.

for_test(DateTime.new(1582,10,14,0,0,0,0,Date::GREGORIAN)) do |t|
         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DateTime: Prefer `Time` over `DateTime`.

assert_equal_with_offset(DateTime.new(1582,10,14,0,0,0,0,Date::GREGORIAN), dt)
                         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DateTime: Prefer `Time` over `DateTime`.

DateTime.strptime(normalize_input(string, format), format, Date::ITALY)
^ Style/DateTime: Prefer `Time` over `DateTime`.
