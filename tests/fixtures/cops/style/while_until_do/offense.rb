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

while total_width >
      minimum_width do
                    ^^ Style/WhileUntilDo: Do not use `do` with multi-line `while`.
  pack_column
end

while (
  needs_more &&
  still_running
) do
  ^^ Style/WhileUntilDo: Do not use `do` with multi-line `while`.
  buf = stack.pop
end

while !found_assoc &&
      segments.size > 0 do
                        ^^ Style/WhileUntilDo: Do not use `do` with multi-line `while`.
  find_assoc
end

until versions.length == 0 ||
      versions.shift <= previous_version do
                                         ^^ Style/WhileUntilDo: Do not use `do` with multi-line `until`.
  history_string += version_lines.shift
end

while !block.empty? &&
      react_wunderbar_free([block.first]) do
                                          ^^ Style/WhileUntilDo: Do not use `do` with multi-line `while`.
  prolog << process(block.shift)
end

while iter do # Keep spinning until told otherwise
           ^^ Style/WhileUntilDo: Do not use `do` with multi-line `while`.
  iter += 1
end

until i > 9 do i += 1
            ^^ Style/WhileUntilDo: Do not use `do` with multi-line `until`.
end

while i < 3 do i += 1
            ^^ Style/WhileUntilDo: Do not use `do` with multi-line `while`.
end
