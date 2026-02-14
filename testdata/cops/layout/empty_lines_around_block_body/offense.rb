items.each do |x|

^ Layout/EmptyLinesAroundBlockBody: Extra empty line detected at block body beginning.
  puts x

^ Layout/EmptyLinesAroundBlockBody: Extra empty line detected at block body end.
end
[1, 2].map do |x|

^ Layout/EmptyLinesAroundBlockBody: Extra empty line detected at block body beginning.
  x * 2

^ Layout/EmptyLinesAroundBlockBody: Extra empty line detected at block body end.
end
foo.select do |x|

^ Layout/EmptyLinesAroundBlockBody: Extra empty line detected at block body beginning.
  x > 1
end
