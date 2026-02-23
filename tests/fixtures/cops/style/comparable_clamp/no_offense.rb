x.clamp(low, high)
if x < low
  low
else
  x
end

if x == 1
  :one
elsif x == 2
  :two
else
  :other
end

# Multi-branch if/elsif chain where branches have different computations
# is NOT a clamp pattern
if sec < 1
  format_ms(sec)
elsif sec < 60
  format_sec(sec)
elsif sec < 3600
  format_min(sec)
else
  format_hr(sec)
end
