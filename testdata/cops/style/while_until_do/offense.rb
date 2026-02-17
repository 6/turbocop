while cond do
           ^^ Style/WhileUntilDo: Do not use `do` with multi-line `while`.
end

until cond do
           ^^ Style/WhileUntilDo: Do not use `do` with multi-line `until`.
end

while x.any? do
             ^^ Style/WhileUntilDo: Do not use `do` with multi-line `while`.
  do_something(x.pop)
end
