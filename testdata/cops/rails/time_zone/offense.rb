Time.now
^^^^^^^^ Rails/TimeZone: Use `Time.zone.now` instead of `Time.now`.

x = Time.now
    ^^^^^^^^ Rails/TimeZone: Use `Time.zone.now` instead of `Time.now`.

if Time.now > deadline
   ^^^^^^^^ Rails/TimeZone: Use `Time.zone.now` instead of `Time.now`.
  puts "expired"
end
