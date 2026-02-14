case x
when 1
  first
when 2
  second
when 1
     ^ Lint/DuplicateCaseCondition: Duplicate `when` condition detected.
  third
end