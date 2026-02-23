case :a
when 1 == 2
  foo
when 1 == 1
  bar
else
  baz
end

case x
when 1
  foo
end

v = case
    when x.a
      1
    when x.b
      return 2
    end

return case
       when foo
         1
       else
         2
       end

do_something case
             when foo
               1
             else
               2
             end
