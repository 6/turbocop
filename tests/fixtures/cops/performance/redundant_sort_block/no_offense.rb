[3, 1, 2].sort
[3, 1, 2].sort { |a, b| b <=> a }
[3, 1, 2].sort_by { |x| x.name }
arr.sort { |a, b| a.name <=> b.name }
arr.sort_by(&:name)
# Numbered params in reverse order — not redundant
arr.sort { _2 <=> _1 }
# Numbered params with property access — not redundant
arr.sort { _1.name <=> _2.name }
