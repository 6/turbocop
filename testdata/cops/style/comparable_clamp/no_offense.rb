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
