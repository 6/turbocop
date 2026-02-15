class Foo
  x = 1
end

def bar
  y = 2
end

if true
  z = 3
end

while true
  a = 1
end

module Baz
  CONST = 1
end

def single_line; end

# Block body indented from line start, not from do/{
items.each do |item|
  process(item)
end

settings index: index_preset(refresh_interval: '30s') do
  field(:id, type: 'long')
end

[1, 2].map { |x|
  x * 2
}

case x
when 1
  do_something
when 2
  do_other
end
