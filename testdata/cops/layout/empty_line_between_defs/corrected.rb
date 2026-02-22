class Foo
  def bar
    1
  end

  def baz
    2
  end

  def qux
    3
  end

  def quux
    4
  end

  def corge
    5
  end
end

# Two defs separated only by comments (no blank lines)
class Grault
  def alpha
    1
  end
  # comment about bravo

  def bravo
    2
  end
  # first comment
  # second comment

  def charlie
    3
  end
  # inline comment on end

  def delta
    4
  end
end

# Too many blank lines between defs
class Garply
  def one
    1
  end


  def two
    2
  end



  def three
    3
  end
end

# Multi-line def after single-line def without blank line
class Waldo
  def initialize(app) @app = app end

  def call(env)
    @app.call(env)
  end
end

# Multi-line def after multiple adjacent single-line defs without blank line
class Fred
  def alpha; 1 end
  def bravo; 2 end

  def charlie
    3
  end
end

# Too many blank lines after single-line def
class Plugh
  def short; 1 end


  def long
    2
  end
end
