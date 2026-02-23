class Foo
  def foo; end
end

class Bar
  bar = 1
end

class Baz < Base
  include Mod
end

class Empty; end

# Single-line class definitions are not offenses
class Foo; def foo; end; end
class Bar; bar = 1; end
class << self; self; end
class << obj; attr_accessor :name; end
