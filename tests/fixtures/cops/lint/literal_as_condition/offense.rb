if true
   ^^^^ Lint/LiteralAsCondition: Literal `true` appeared as a condition.
  x = 1
end
if 42
   ^^ Lint/LiteralAsCondition: Literal `42` appeared as a condition.
  x = 2
end
while false
      ^^^^^ Lint/LiteralAsCondition: Literal `false` appeared as a condition.
  break
end
case true
     ^^^^ Lint/LiteralAsCondition: Literal `true` appeared as a condition.
when 1 then "one"
end
case 42
     ^^ Lint/LiteralAsCondition: Literal `42` appeared as a condition.
when 1 then "one"
end
case nil
     ^^^ Lint/LiteralAsCondition: Literal `nil` appeared as a condition.
when NilClass then "nil"
end

# Ternary with literal condition
nil ? top : bar
^^^ Lint/LiteralAsCondition: Literal `nil` appeared as a condition.
false ? top : bar
^^^^^ Lint/LiteralAsCondition: Literal `false` appeared as a condition.
42 ? top : bar
^^ Lint/LiteralAsCondition: Literal `42` appeared as a condition.

# Modifier if/unless with literal condition
top if 42
       ^^ Lint/LiteralAsCondition: Literal `42` appeared as a condition.
top unless 42
           ^^ Lint/LiteralAsCondition: Literal `42` appeared as a condition.

# String, symbol, array, hash, regex literals in if
if "hello"
   ^^^^^^^ Lint/LiteralAsCondition: Literal `"hello"` appeared as a condition.
  x = 1
end
if :sym
   ^^^^ Lint/LiteralAsCondition: Literal `:sym` appeared as a condition.
  x = 1
end
if [1]
   ^^^ Lint/LiteralAsCondition: Literal `[1]` appeared as a condition.
  x = 1
end
if {}
   ^^ Lint/LiteralAsCondition: Literal `{}` appeared as a condition.
  x = 1
end
if 2.0
   ^^^ Lint/LiteralAsCondition: Literal `2.0` appeared as a condition.
  x = 1
end

# Truthy literal on lhs of &&
if 1 && x
   ^ Lint/LiteralAsCondition: Literal `1` appeared as a condition.
  top
end

# Falsey literal on lhs of ||
if nil || x
   ^^^ Lint/LiteralAsCondition: Literal `nil` appeared as a condition.
  top
end
if false || x
   ^^^^^ Lint/LiteralAsCondition: Literal `false` appeared as a condition.
  top
end

# Standalone && and || (not inside if)
1 && x
^ Lint/LiteralAsCondition: Literal `1` appeared as a condition.
nil || x
^^^ Lint/LiteralAsCondition: Literal `nil` appeared as a condition.

# !literal
!42
 ^^ Lint/LiteralAsCondition: Literal `42` appeared as a condition.
!nil
 ^^^ Lint/LiteralAsCondition: Literal `nil` appeared as a condition.

# not(literal)
not(42)
    ^^ Lint/LiteralAsCondition: Literal `42` appeared as a condition.

# case without predicate: when with all-literal conditions
case
when 1 then top
     ^ Lint/LiteralAsCondition: Literal `1` appeared as a condition.
end
case
when :sym then top
     ^^^^ Lint/LiteralAsCondition: Literal `:sym` appeared as a condition.
end
case
when "str" then top
     ^^^^^ Lint/LiteralAsCondition: Literal `"str"` appeared as a condition.
end

# !literal in if condition
if !42
    ^^ Lint/LiteralAsCondition: Literal `42` appeared as a condition.
  top
end

# Nested: truthy literal in complex condition with !
if x && !(1 && a) && y && z
          ^ Lint/LiteralAsCondition: Literal `1` appeared as a condition.
  top
end

# until with literal (not false)
until nil
      ^^^ Lint/LiteralAsCondition: Literal `nil` appeared as a condition.
  top
end
until 42
      ^^ Lint/LiteralAsCondition: Literal `42` appeared as a condition.
  top
end
