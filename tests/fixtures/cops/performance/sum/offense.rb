[1, 2, 3].inject(:+)
          ^^^^^^ Performance/Sum: Use `sum` instead of `inject(:+)`.
[1, 2, 3].inject(0, :+)
          ^^^^^^^^^^^^^ Performance/Sum: Use `sum` instead of `inject(0, :+)`.
[1, 2, 3].reduce(:+)
          ^^^^^^ Performance/Sum: Use `sum` instead of `reduce(:+)`.
[1, 2, 3].reduce(0, :+)
          ^^^^^^^^^^^^^ Performance/Sum: Use `sum` instead of `reduce(0, :+)`.
items.reduce(:+)
      ^^^^^^ Performance/Sum: Use `sum` instead of `reduce(:+)`.
body.map(&:bytesize).reduce(0, :+)
                     ^^^^^^ Performance/Sum: Use `sum` instead of `reduce(0, :+)`.
items.inject(0) { |sum, n| sum + n }
      ^^^^^^ Performance/Sum: Use `sum` instead of `inject(0) { |acc, elem| acc + elem }`.
items.inject(0) { |sum, n| n + sum }
      ^^^^^^ Performance/Sum: Use `sum` instead of `inject(0) { |acc, elem| acc + elem }`.
items.reduce(0) { |acc, val| acc + val }
      ^^^^^^ Performance/Sum: Use `sum` instead of `reduce(0) { |acc, elem| acc + elem }`.
items.inject { |sum, x| sum + x }
      ^^^^^^ Performance/Sum: Use `sum` instead of `inject { |acc, elem| acc + elem }`.
items.reduce { |sum, x| sum + x }
      ^^^^^^ Performance/Sum: Use `sum` instead of `reduce { |acc, elem| acc + elem }`.
items.inject(10) { |sum, n| sum + n }
      ^^^^^^ Performance/Sum: Use `sum(10)` instead of `inject(10) { |acc, elem| acc + elem }`.
values.inject(0.0) { |sum, v| sum + v }
       ^^^^^^ Performance/Sum: Use `sum(0.0)` instead of `inject(0.0) { |acc, elem| acc + elem }`.
items.inject(init, :+)
      ^^^^^^ Performance/Sum: Use `sum(init)` instead of `inject(init, :+)`.
items.reduce(init, :+)
      ^^^^^^ Performance/Sum: Use `sum(init)` instead of `reduce(init, :+)`.
items.inject(10, :+)
      ^^^^^^ Performance/Sum: Use `sum(10)` instead of `inject(10, :+)`.
items.inject(&:+)
      ^^^^^^ Performance/Sum: Use `sum` instead of `inject(&:+)`, unless calling `inject(&:+)` on an empty array.
items.reduce(&:+)
      ^^^^^^ Performance/Sum: Use `sum` instead of `reduce(&:+)`, unless calling `reduce(&:+)` on an empty array.
items.inject(0, &:+)
      ^^^^^^ Performance/Sum: Use `sum` instead of `inject(0, &:+)`.
items.reduce(0, &:+)
      ^^^^^^ Performance/Sum: Use `sum` instead of `reduce(0, &:+)`.
items.map { |x| x.value }.sum
      ^^^ Performance/Sum: Use `sum { ... }` instead of `map { ... }.sum`.
items.collect { |x| x ** 2 }.sum
      ^^^^^^^ Performance/Sum: Use `sum { ... }` instead of `collect { ... }.sum`.
items.map(&:count).sum
      ^^^ Performance/Sum: Use `sum { ... }` instead of `map { ... }.sum`.
items.map { |x| x.value }.sum(10)
      ^^^ Performance/Sum: Use `sum(10) { ... }` instead of `map { ... }.sum(10)`.
