if cond then
         ^^^^ Style/MultilineIfThen: Do not use `then` for multi-line `if`.
end

if cond1
  a
elsif cond2 then
            ^^^^ Style/MultilineIfThen: Do not use `then` for multi-line `elsif`.
  b
end

unless cond then
            ^^^^ Style/MultilineIfThen: Do not use `then` for multi-line `unless`.
end
