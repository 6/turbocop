[1, 2, 3].filter_map { |x| x > 1 ? x * 2 : nil }
[1, 2, 3].select { |x| x > 1 }
[1, 2, 3].map { |x| x * 2 }
arr.select { |x| x > 1 }.each { |x| puts x }
arr.select { |x| x > 1 }.count
ary.do_something.select(&:present?).stranger.map(&:to_i).max
ary.select { |o| o.present? }.stranger.map { |o| o.to_i }
ary.do_something.select(key: value).map(&:to_i)
