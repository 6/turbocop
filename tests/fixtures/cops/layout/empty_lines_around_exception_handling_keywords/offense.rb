begin
  do_something

^ Layout/EmptyLinesAroundExceptionHandlingKeywords: Extra empty line detected before the `rescue`.
rescue

^ Layout/EmptyLinesAroundExceptionHandlingKeywords: Extra empty line detected after the `rescue`.
  handle_error
end

begin
  something

^ Layout/EmptyLinesAroundExceptionHandlingKeywords: Extra empty line detected before the `ensure`.
ensure

^ Layout/EmptyLinesAroundExceptionHandlingKeywords: Extra empty line detected after the `ensure`.
  cleanup
end

begin
  recover
rescue=>e

^ Layout/EmptyLinesAroundExceptionHandlingKeywords: Extra empty line detected after the `rescue`.
  handle_error
end

begin
  work

^ Layout/EmptyLinesAroundExceptionHandlingKeywords: Extra empty line detected before the `rescue`.
rescue(EOFError)
end

def parse_single_json_value(s)

^ Layout/EmptyLinesAroundExceptionHandlingKeywords: Extra empty line detected before the `rescue`.
  Flor::ConfExecutor.interpret_line(s) rescue nil
end

def attd
  data['atts'].inject({}) { |h, (k, v)| h[k] = v if k; h }

^ Layout/EmptyLinesAroundExceptionHandlingKeywords: Extra empty line detected before the `rescue`.
rescue; []
end

def attl
  data['atts'].inject([]) { |a, (k, v)| a << v if k == nil; a }

^ Layout/EmptyLinesAroundExceptionHandlingKeywords: Extra empty line detected before the `rescue`.
rescue; []
end

def parse(s, opts={})

^ Layout/EmptyLinesAroundExceptionHandlingKeywords: Extra empty line detected before the `rescue`.
  do_parse(s, opts || {}) rescue nil
end
