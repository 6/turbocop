if x == 42
  do_something
end

if x = y
  do_something
end

if x = some_method
  do_something
end

while line = gets
  process(line)
end

if values = [1, x, y]
  do_something
end

if options = { x => :y }
  do_something
end

if options = { x: y }
  do_something
end

if has_terminal?(@regexps[gr = '\A[
    ]'] ||= Regexp.new(gr), :regexp, index)
  do_something
end
