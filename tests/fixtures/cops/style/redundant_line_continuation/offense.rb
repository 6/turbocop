foo(1, \
       ^ Style/RedundantLineContinuation: Redundant line continuation.
  2)

x = 1 + \
        ^ Style/RedundantLineContinuation: Redundant line continuation.
  2

[1, \
    ^ Style/RedundantLineContinuation: Redundant line continuation.
 2]

if children \
            ^ Style/RedundantLineContinuation: Redundant line continuation.
  .reject { |c| c }
end

obj.elements['BuildAction'] \
                            ^ Style/RedundantLineContinuation: Redundant line continuation.
  .elements['Next']

foo(bar) \
         ^ Style/RedundantLineContinuation: Redundant line continuation.
  .baz

foo \
    ^ Style/RedundantLineContinuation: Redundant line continuation.
  .bar \
       ^ Style/RedundantLineContinuation: Redundant line continuation.
    .baz

foo&. \
      ^ Style/RedundantLineContinuation: Redundant line continuation.
  bar

foo do \
       ^ Style/RedundantLineContinuation: Redundant line continuation.
  bar
end

class Foo \
          ^ Style/RedundantLineContinuation: Redundant line continuation.
end
