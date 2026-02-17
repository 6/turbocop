case foo
when bar then do_something
end

case foo
when bar
end

case foo
when bar
  do_something
end

case condition
when foo then {
    key: 'value'
  }
end

case foo
when bar then do_something
              do_another_thing
end

case foo
when bar,
     baz then do_something
end
