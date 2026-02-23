class Foo
  attr_reader :bar1, :bar2, :bar3

  attr_accessor :quux

  attr_writer :baz
end

class Bar
  attr_reader :x
end

# Accessors separated by other method calls - not grouped
class WithAnnotations
  extend T::Sig

  annotation_method :one
  attr_reader :one

  annotation_method :two
  attr_reader :two
end

# Accessors separated by method defs
class WithDefs
  attr_reader :one

  def foo; end

  attr_reader :two
end

# Accessors in different visibility scopes
class WithScopes
  attr_reader :public_one

  private

  attr_reader :private_one
end
