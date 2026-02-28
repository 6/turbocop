[1, 2, 3].detect { |x| x > 1 }
[1, 2, 3].first
arr.find { |x| x > 1 }
arr.select { |x| x > 1 }
[1, 2, 3].select { |x| x > 1 }.first(n)
[1, 2, 3].select { |x| x > 1 }.last(n)
adapter.select.first
adapter.select('something').first
adapter.lazy.select { 'something' }.first
adapter.lazy.select(&:even?).first
items&.select { |x| x.valid? }&.first
items&.filter { |x| x > 0 }&.last
items.select { _1.active? }.first
items.filter { _1 > 0 }.last
items.select { it.active? }.first
