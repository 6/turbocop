[].each do |o|
  if o == 1
  ^^ Style/Next: Use `next` to skip iteration.
    puts o
    puts o
    puts o
  end
end

3.downto(1) do
  if true
  ^^ Style/Next: Use `next` to skip iteration.
    a = 1
    b = 2
    c = 3
  end
end

items.map do |item|
  unless item.nil?
  ^^^^^^ Style/Next: Use `next` to skip iteration.
    process(item)
    transform(item)
    finalize(item)
  end
end
