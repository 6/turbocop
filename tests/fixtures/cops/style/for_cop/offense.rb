for n in [1, 2, 3] do
^^^^^^^^^^^^^^^^^^^^^ Style/For: Prefer `each` over `for`.
  puts n
end

for x in items
^^^^^^^^^^^^^^ Style/For: Prefer `each` over `for`.
  process(x)
end

for i in 1..10
^^^^^^^^^^^^^^ Style/For: Prefer `each` over `for`.
  puts i
end
