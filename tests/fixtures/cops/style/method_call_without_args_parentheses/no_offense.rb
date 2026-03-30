top.test
foo.bar(arg)
Test()
not(something)
puts 1, 2
obj.baz

# Bracket method calls (operator methods)
Set[]
Hash[]

# Lambda call syntax
thing.()
foo(&block)
items.map!(&:name)

# Same-name local variable assignment (disambiguation)
test = test()
name = name()

# Same-name shorthand assignment
test ||= test()
name &&= name()
test += test()

# Same-name parallel (mass) assignment
one, test = 1, test()

# Same-name complex assignment
test = begin
  case a
  when b
    c = test() if d
  end
end

# Default argument assignment
def foo(test = test())
end

def bar(name = name(), status = status())
end

# it() without receiver inside a block (no explicit params)
0.times { it() }

0.times do
  it()
  it = 1
  it
end
