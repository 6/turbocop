begin
  foo
rescue => e
  bar
end

begin
  foo
ensure
  bar
end

def baz
  foo
rescue => e
  bar
end

# Assignment begin: rescue aligns with variable, not begin
digest_size = begin
  Base64.strict_decode64(sha256[1].strip).length
rescue ArgumentError
  raise "Invalid Digest"
end

matched_ip = begin
  IPAddr.new(ip_query)
rescue IPAddr::Error
  nil
ensure
  cleanup
end

# Single-line begin/rescue (modifier-like)
begin; do_something; rescue LoadError; end

# Single-line begin/rescue in assignment
result = begin; compute; rescue; nil; end

# Single-line begin/rescue inside block
items.each { begin; process; rescue; nil; end }
