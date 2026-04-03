begin
  do_something
rescue
  handle_error
end

begin
  something
ensure
  cleanup
end

begin
  recover
rescue=>e
  handle_error
end

begin
  work
rescue(EOFError)
end

def parse_single_json_value(s)
  Flor::ConfExecutor.interpret_line(s) rescue nil
end

def attd
  data['atts'].inject({}) { |h, (k, v)| h[k] = v if k; h }
rescue; []
end

def attl
  data['atts'].inject([]) { |a, (k, v)| a << v if k == nil; a }
rescue; []
end

def parse(s, opts={})
  do_parse(s, opts || {}) rescue nil
end

handler = -> do
  begin
    work
  rescue => e
    handle
  end

  return_value
ensure
  cleanup
end
