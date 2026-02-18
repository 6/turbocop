def some_method
  foo = 1
  puts foo
  1.times do |bar|
  end
end
def some_method
  foo = 1
  puts foo
  1.times do
    foo = 2
  end
end
def some_method
  _ = 1
  puts _
  1.times do |_|
  end
end
def some_method
  _foo = 1
  puts _foo
  1.times do |_foo|
  end
end

# Variables from sibling blocks should not be treated as outer locals
def sibling_blocks
  [1].each { |x| y = x + 1; puts y }
  [2].each { |y| puts y }
end

# Variables from sibling lambdas should not leak
def sibling_lambdas
  a = lambda { |n| n = n.to_s; puts n }
  b = lambda { |n| puts n }
  a.call(1)
  b.call(2)
end

# Variables defined inside a block should not shadow in sibling block
class MyClass
  scope :secured, ->(guardian) { ids = guardian.secure_ids; puts ids }
  scope :with_parents, ->(ids) { where(ids) }
end

# Nested block variables should not leak to outer scope
def nested_blocks
  items.each do |item|
    item.children.each { |child| value = child.name; puts value }
  end
  other_items.each do |other|
    other.parts.each { |value| puts value }
  end
end

# Different branches of case/when - not flagged
def different_branches
  case filter
  when "likes-min"
    value = values.last
    value if value =~ /\A\d+\z/
  when "order"
    values.flat_map { |value| value.split(",") }
  end
end

# Class-level begin block vars don't shadow method-level block params
class MyTranslator
  MAPPING =
    begin
      from = "abc"
      to = "xyz"
      from.chars.zip(to.chars)
    end

  def translate(name)
    MAPPING.each { |from, to| name.gsub!(from, to) }
    name
  end
end
