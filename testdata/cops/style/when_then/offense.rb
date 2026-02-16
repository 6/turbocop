case a
when b; c
      ^ Style/WhenThen: Do not use `when b;`. Use `when b then` instead.
end

case a
when b, c; d
         ^ Style/WhenThen: Do not use `when b, c;`. Use `when b, c then` instead.
end

case x
when 1; "one"
      ^ Style/WhenThen: Do not use `when 1;`. Use `when 1 then` instead.
end
