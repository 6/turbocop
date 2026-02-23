arr.select { |x| x > 1 }.any?
    ^^^^^^ Style/RedundantFilterChain: Use `any?` instead of `select.any?`.

arr.filter { |x| x > 1 }.empty?
    ^^^^^^ Style/RedundantFilterChain: Use `none?` instead of `filter.empty?`.

arr.find_all { |x| x > 1 }.none?
    ^^^^^^^^ Style/RedundantFilterChain: Use `none?` instead of `find_all.none?`.
