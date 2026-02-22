class Foo
  def bar
    1
  end
  def baz
  ^^^ Layout/EmptyLineBetweenDefs: Use empty lines between method definitions.
    2
  end
  def qux
  ^^^ Layout/EmptyLineBetweenDefs: Use empty lines between method definitions.
    3
  end

  def quux
    4
  end
  def corge
  ^^^ Layout/EmptyLineBetweenDefs: Use empty lines between method definitions.
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
  ^^^ Layout/EmptyLineBetweenDefs: Use empty lines between method definitions.
    2
  end
  # first comment
  # second comment
  def charlie
  ^^^ Layout/EmptyLineBetweenDefs: Use empty lines between method definitions.
    3
  end
  # inline comment on end
  def delta
  ^^^ Layout/EmptyLineBetweenDefs: Use empty lines between method definitions.
    4
  end
end

# Too many blank lines between defs
class Garply
  def one
    1
  end


  def two
  ^^^ Layout/EmptyLineBetweenDefs: Expected 1 empty line between method definitions; found 2.
    2
  end



  def three
  ^^^ Layout/EmptyLineBetweenDefs: Expected 1 empty line between method definitions; found 3.
    3
  end
end

# Multi-line def after single-line def without blank line
class Waldo
  def initialize(app) @app = app end
  def call(env)
  ^^^ Layout/EmptyLineBetweenDefs: Use empty lines between method definitions.
    @app.call(env)
  end
end

# Multi-line def after multiple adjacent single-line defs without blank line
class Fred
  def alpha; 1 end
  def bravo; 2 end
  def charlie
  ^^^ Layout/EmptyLineBetweenDefs: Use empty lines between method definitions.
    3
  end
end

# Too many blank lines after single-line def
class Plugh
  def short; 1 end


  def long
  ^^^ Layout/EmptyLineBetweenDefs: Expected 1 empty line between method definitions; found 2.
    2
  end
end
