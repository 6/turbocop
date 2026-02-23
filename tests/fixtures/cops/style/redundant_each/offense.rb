array.each.each { |v| do_something(v) }
     ^^^^^ Style/RedundantEach: Remove redundant `each`.

array.each.each_with_index { |v| do_something(v) }
     ^^^^^ Style/RedundantEach: Remove redundant `each`.

array.each.each_with_object([]) { |v, o| do_something(v, o) }
     ^^^^^ Style/RedundantEach: Remove redundant `each`.
