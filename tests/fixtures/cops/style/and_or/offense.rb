if a and b
     ^^^ Style/AndOr: Use `&&` instead of `and`.
  do_something
end

if a or b
     ^^ Style/AndOr: Use `||` instead of `or`.
  do_something
end

while x and y
        ^^^ Style/AndOr: Use `&&` instead of `and`.
  do_something
end

# FN fix: and/or inside parentheses within conditions
until (x or y)
         ^^ Style/AndOr: Use `||` instead of `or`.
  do_something
end

if (a and b)
      ^^^ Style/AndOr: Use `&&` instead of `and`.
  do_something
end

do_something unless (a or b)
                       ^^ Style/AndOr: Use `||` instead of `or`.

until (x or y or z)
         ^^ Style/AndOr: Use `||` instead of `or`.
              ^^ Style/AndOr: Use `||` instead of `or`.
  do_something
end

if foo and (bar or baz)
       ^^^ Style/AndOr: Use `&&` instead of `and`.
                ^^ Style/AndOr: Use `||` instead of `or`.
  do_something
end

if (a and b) or (c and d)
      ^^^ Style/AndOr: Use `&&` instead of `and`.
             ^^ Style/AndOr: Use `||` instead of `or`.
                   ^^^ Style/AndOr: Use `&&` instead of `and`.
  do_something
end
