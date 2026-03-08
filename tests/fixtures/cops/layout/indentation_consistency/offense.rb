def foo
  x = 1
    y = 2
    ^^^ Layout/IndentationConsistency: Inconsistent indentation detected.
end

class Bar
  a = 1
      b = 2
      ^^^ Layout/IndentationConsistency: Inconsistent indentation detected.
end

module Baz
  c = 1
        d = 2
        ^^^ Layout/IndentationConsistency: Inconsistent indentation detected.
end

if cond
 func
  func
  ^^^^ Layout/IndentationConsistency: Inconsistent indentation detected.
end

if cond
  func1
else
 func2
  func2
  ^^^^^ Layout/IndentationConsistency: Inconsistent indentation detected.
end

unless cond
 func
  func
  ^^^^ Layout/IndentationConsistency: Inconsistent indentation detected.
end

case a
when b
 c
    d
    ^ Layout/IndentationConsistency: Inconsistent indentation detected.
end

while cond
 func
  func
  ^^^^ Layout/IndentationConsistency: Inconsistent indentation detected.
end

until cond
 func
  func
  ^^^^ Layout/IndentationConsistency: Inconsistent indentation detected.
end

for var in 1..10
 func
func
^^^^ Layout/IndentationConsistency: Inconsistent indentation detected.
end

begin
 func1
   func2
   ^^^^^ Layout/IndentationConsistency: Inconsistent indentation detected.
end
