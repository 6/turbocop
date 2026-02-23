if (x > 1)
   ^ Style/ParenthesesAroundCondition: Don't use parentheses around the condition of an `if`.
  do_something
end

while (x > 1)
      ^ Style/ParenthesesAroundCondition: Don't use parentheses around the condition of a `while`.
  do_something
end

until (x > 1)
      ^ Style/ParenthesesAroundCondition: Don't use parentheses around the condition of an `until`.
  do_something
end

if (x)
   ^ Style/ParenthesesAroundCondition: Don't use parentheses around the condition of an `if`.
  bar
end

while (running)
      ^ Style/ParenthesesAroundCondition: Don't use parentheses around the condition of a `while`.
  process
end

do_something unless (condition)
                    ^ Style/ParenthesesAroundCondition: Don't use parentheses around the condition of an `unless`.

result = foo if (bar)
                ^ Style/ParenthesesAroundCondition: Don't use parentheses around the condition of an `if`.

run_task until (done)
               ^ Style/ParenthesesAroundCondition: Don't use parentheses around the condition of an `until`.
