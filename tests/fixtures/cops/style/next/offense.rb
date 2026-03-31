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

# Last statement in multi-statement block body
[].each do |o|
  x = 1
  if o == 1
  ^^ Style/Next: Use `next` to skip iteration.
    puts o
    puts o
    puts o
  end
end

# for loop with if/unless as sole body
for post in items
  unless post.nil?
  ^^^^^^ Style/Next: Use `next` to skip iteration.
    process(post)
    transform(post)
    finalize(post)
  end
end

# for loop with last-statement pattern
for item in items
  x = process(item)
  if item.valid?
  ^^ Style/Next: Use `next` to skip iteration.
    transform(item)
    save(item)
    finalize(item)
  end
end

# while loop
while running
  if test
  ^^ Style/Next: Use `next` to skip iteration.
    something
    something
    something
  end
end

# until loop
until finished
  if test
  ^^ Style/Next: Use `next` to skip iteration.
    something
    something
    something
  end
end

# loop method
loop do
  if test
  ^^ Style/Next: Use `next` to skip iteration.
    something
    something
    something
  end
end
