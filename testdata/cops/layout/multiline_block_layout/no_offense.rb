blah do |i|
  foo(i)
  bar(i)
end

blah { |i|
  foo(i)
  bar(i)
}

items.each { |x| puts x }

[1, 2].map do |x|
  x * 2
end

# Block with rescue — body on next line (not same line as do)
urls.reject do |url|
  host = parse(url)
  check(host)
rescue StandardError
  true
end

# Block with ensure — body on next line
around_action do |_controller, block|
  block.call
ensure
  cleanup
end

# Block with rescue, no block params
items.each do
  process_item
rescue => e
  log(e)
end

# Block with rescue — brace style
data.map { |x|
  transform(x)
rescue TypeError
  nil
}
