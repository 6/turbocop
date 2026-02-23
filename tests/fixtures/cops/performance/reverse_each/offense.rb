[1, 2, 3].reverse.each { |x| puts x }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/ReverseEach: Use `reverse_each` instead of `reverse.each`.
[1, 2, 3].reverse.each do |x|
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/ReverseEach: Use `reverse_each` instead of `reverse.each`.
  puts x
end
arr.reverse.each { |item| process(item) }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/ReverseEach: Use `reverse_each` instead of `reverse.each`.
