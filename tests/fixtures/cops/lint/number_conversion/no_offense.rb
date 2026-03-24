Integer('10', 10)
Float('10.2')
Complex('10')
Rational('1/3')
42.to_i
42.0.to_f
# Kernel conversion methods as receivers should be skipped
Integer(var, 10).to_f
Float(var).to_i
Complex(var).to_f
# Already-converted values: .to_f on result of Integer() is fine
Integer(var, 10).to_f
# Symbol form with multiple arguments should not be flagged
delegate :to_f, to: :description, allow_nil: true
# IgnoredClasses (defaults: Time, DateTime)
Time.now.to_i
Time.now.to_f
DateTime.new(2012, 8, 29, 22, 35, 0).to_i
Time.now.to_datetime.to_i
# Simple Time and DateTime are ignored
Time.zone.now.to_i
DateTime.current.to_f
# Block-pass with additional arguments should not be flagged
foo.map(x, &:to_i)
foo.reduce(0, &:to_f)
# Rooted constant paths (::Time, ::DateTime) should also be ignored
::Time.now.to_f
::Time.now.to_i
::Time.local(2010).utc_offset.to_r
::DateTime.new(2012, 8, 29).to_i
