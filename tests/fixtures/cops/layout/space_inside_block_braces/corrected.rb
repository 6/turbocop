[1, 2].each { |x| puts x }
[1, 2].map { |x| x * 2 }
foo.select { |x| x > 1 }
escape_html = ->(str) { str.gsub("&", "&amp;") }
has_many :versions, -> { order("id ASC") }, class_name: "Foo"
action = -> { puts "hello" }

p(class: 'intro') { "
hello
"}

p { "
hello
"}

p(class: 'conclusion') { "
hello
"}

p(class: 'legend') { "
hello
"}

audit_options { {
  foo: :bar
}}

let(:domains) { [
  { domain: 'example.com' }
]}

let(:source) { <<-CODE
body
CODE
}

before { FlavourSaver.register_helper(:repeat) do |a, &block|
  a.times { block.call }
end}

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
