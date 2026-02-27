Array.new(5) { |i| i * 2 }
5.times { |i| puts i }
[1, 2, 3].map { |x| x * 2 }
5.times.each { |i| puts i }
5.times.select { |i| i > 2 }
Factory.times(10, :post).map(&:topic)
Builder.times(5, :user).collect { |u| u.name }
times.map { |time| time * 1_000 }
times.collect { |t| t.to_s }
