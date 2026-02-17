case foo
when bar then
         ^^^^ Style/MultilineWhenThen: Do not use `then` for multiline `when` statement.
end

case foo
when bar then
         ^^^^ Style/MultilineWhenThen: Do not use `then` for multiline `when` statement.
  do_something
end

case foo
when bar then
         ^^^^ Style/MultilineWhenThen: Do not use `then` for multiline `when` statement.
  do_something1
  do_something2
end

case foo
when bar, baz then
              ^^^^ Style/MultilineWhenThen: Do not use `then` for multiline `when` statement.
end
