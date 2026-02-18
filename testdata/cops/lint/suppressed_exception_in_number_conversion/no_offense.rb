Integer(arg, exception: false)
Float(arg, exception: false)
Integer(arg) rescue :fallback
something rescue nil
x = Integer(arg)
# Rescue with non-ArgumentError/TypeError should not trigger
begin
  Rational(raw)
rescue ZeroDivisionError
  nil
end
begin
  Integer(arg)
rescue NameError
  nil
end
