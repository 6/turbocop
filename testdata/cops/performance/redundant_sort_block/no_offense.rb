[3, 1, 2].sort
[3, 1, 2].sort { |a, b| b <=> a }
[3, 1, 2].sort_by { |x| x.name }
arr.sort { |a, b| a.name <=> b.name }
arr.sort_by(&:name)
