array.map { |x| foo(x) }
array.map(&:foo)
array.each { |x| process(x) }
array.map(&block)
method(:foo)
