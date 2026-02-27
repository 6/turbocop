case a
when b then c
end

case e
when f
  g
end

case value
when cond1
end

case x
when 1 then "one"
when 2 then "two"
end

# Comment with semicolon between when condition and body
case value
when /pattern/
  # Handles text/html; charset="GB2312"
  process(value)
end

case input
when String
  # key; value pairs
  parse(input)
end
