Time.now
     ^^^ Rails/TimeZone: Use `Time.zone.now` instead of `Time.now`.

x = Time.now
         ^^^ Rails/TimeZone: Use `Time.zone.now` instead of `Time.now`.

if Time.now > deadline
        ^^^ Rails/TimeZone: Use `Time.zone.now` instead of `Time.now`.
  puts "expired"
end

::Time.now
       ^^^ Rails/TimeZone: Use `Time.zone.now` instead of `Time.now`.

Time.now.getutc
     ^^^ Rails/TimeZone: Use `Time.zone.now` instead of `Time.now`.

# .localtime without arguments is NOT safe — RuboCop flags MSG_LOCALTIME
Time.at(time).localtime
     ^^ Rails/TimeZone: Use `Time.zone.at` instead of `Time.at`.

Time.at(@time).localtime.to_s
     ^^ Rails/TimeZone: Use `Time.zone.at` instead of `Time.at`.

Time.now.localtime
     ^^^ Rails/TimeZone: Use `Time.zone.now` instead of `Time.now`.
