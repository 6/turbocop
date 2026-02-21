1.day.from_now
2.hours.ago
Time.now
x + 1.day
Time.zone.now
# Time.now + duration is NOT flagged (only Time.current and Time.zone.now)
Time.now + 1.day
Date.yesterday + 3.days
created_at - 1.minute
3.days - 1.hour
# Method call receivers are NOT matched — only int/float literals
Time.current - expires_days_after_signup.days
Time.zone.now + expires_in.seconds
