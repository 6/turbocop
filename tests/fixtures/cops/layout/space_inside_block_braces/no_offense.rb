[1, 2].each { |x| puts x }
[1, 2].map { |x| x * 2 }
[1, 2].each do |x|
  puts x
end
foo.select { |x| x > 1 }
x = {}
items.each { |x|
  puts x
}
items.map {
  42
}
escape_html = ->(str) { str.gsub("&", "&amp;") }
has_many :versions, -> { order("id ASC") }, class_name: "Foo"
action = -> { puts "hello" }
f = ->(x) { x + 1 }
empty_lambda = ->(x) {}
empty_proc = proc {|x|
}
g = -> {
  42
}

class ForwardingSuperExamples
  def each
    super { |*a| sleep 0.1 ; yield(*a) }
  end

  def union_to_s
    super { ['every ',' or '] }
  end

  def intersect_to_s
    super { ['every ', ' and '] }
  end

  def method_4(&block)
    super { |y| block.call(y) }
  end

  def downto(min)
    super { |dt| yield dt.clear_timezone_offset }
  end

  def step(limit, step = 1)
    super { |dt| yield dt.clear_timezone_offset }
  end

  def upto(max)
    super { |dt| yield dt.clear_timezone_offset }
  end
end
