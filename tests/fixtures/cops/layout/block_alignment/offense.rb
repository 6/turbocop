items.each do |x|
  puts x
  end
  ^^^ Layout/BlockAlignment: Align `end` with the start of the line where the block is defined.
items.map do |x|
  x * 2
    end
    ^^^ Layout/BlockAlignment: Align `end` with the start of the line where the block is defined.
[1, 2].select do |x|
  x > 1
      end
      ^^^ Layout/BlockAlignment: Align `end` with the start of the line where the block is defined.
# FN: end aligns with RHS of assignment (call expression) instead of LHS
answer = prompt.select("Pick one") do |menu|
           menu.choice "A"
         end
         ^^^ Layout/BlockAlignment: Align `end` with the start of the line where the block is defined.
