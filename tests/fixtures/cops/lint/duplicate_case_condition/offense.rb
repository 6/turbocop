case x
when 1
  first
when 2
  second
when 1
     ^ Lint/DuplicateCaseCondition: Duplicate `when` condition detected.
  third
end
case y
when :a
  one
when :b
  two
when :a
     ^^ Lint/DuplicateCaseCondition: Duplicate `when` condition detected.
  three
when :b
     ^^ Lint/DuplicateCaseCondition: Duplicate `when` condition detected.
  four
end
