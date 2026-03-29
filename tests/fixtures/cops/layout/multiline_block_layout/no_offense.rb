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

# Block args on multiple lines — acceptable when single-line would exceed max line length
annotations_collection.zip(other_annotations_collection) do |(_actual_line, actual_annotation),
                                                             (_expected_line, expected_annotation)|
  compare(actual_annotation, expected_annotation)
end

# Lambda with body on next line (no offense)
html = -> {
  content
}

# Lambda with params, body on next line
transform = ->(x) {
  x + 1
}

# Lambda do..end body on next line
action = -> do
  run_task
end

# Single-line lambda (no offense)
double = ->(x) { x * 2 }

# Block args on multiple lines — acceptable when joining would exceed max line length
define_command(:grep) do
  |cmd = read_from_minibuffer("Grep: ",
                              initial_value: CONFIG[:grep_command] + " ")|
  shell_execute(cmd, buffer_name: "*grep*", mode: BacktraceMode)
end
