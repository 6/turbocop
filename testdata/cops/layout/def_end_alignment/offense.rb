def foo
  42
  end
  ^^^ Layout/DefEndAlignment: Align `end` with `def`.
def bar(x)
  x + 1
    end
    ^^^ Layout/DefEndAlignment: Align `end` with `def`.
  def baz
    42
      end
      ^^^ Layout/DefEndAlignment: Align `end` with `def`.
