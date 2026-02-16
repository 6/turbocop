class Foo
  def bar
    super
    do_something_else
  end

  def baz(x)
    x + 1
  end

  def qux
    42
  end

  # Methods with default arguments are not useless (change calling convention)
  def initialize(x = Object.new)
    super
  end

  # Methods with rest args are not useless
  def method_with_rest(*args)
    super
  end

  # Methods with optional keyword args are not useless
  def method_with_kwopt(name: 'default')
    super
  end

  # super with different args than def params is not useless
  def method_with_extra(a, b)
    super(:extra, a, b)
  end

  # super with reordered args is not useless
  def method_reordered(a, b)
    super(b, a)
  end

  # super with fewer args is not useless
  def method_fewer_args(a, b)
    super(a)
  end

  # super with a block adds behavior, not useless
  def create!
    super do |obj|
      obj.save!
    end
  end

  # super with a block (curly braces)
  def process
    super { |x| x.validate }
  end
end
