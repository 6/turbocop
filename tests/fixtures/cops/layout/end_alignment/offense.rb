class Foo
  end
  ^^^ Layout/EndAlignment: Align `end` with `class`.

module Bar
  end
  ^^^ Layout/EndAlignment: Align `end` with `module`.

if true
  1
  end
  ^^^ Layout/EndAlignment: Align `end` with `if`.

while true
  break
  end
  ^^^ Layout/EndAlignment: Align `end` with `while`.

case x
when 1
  :a
  end
  ^^^ Layout/EndAlignment: Align `end` with `case`.
