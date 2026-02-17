some_method(a) { |el| puts el }
some_method(a) do; puts a; end
some_method a do; puts "dev"; end
Foo.bar(a) { |el| puts el }
foo == bar { baz a }
foo ->(a) { bar a }
scope :active, -> { where(status: "active") }
