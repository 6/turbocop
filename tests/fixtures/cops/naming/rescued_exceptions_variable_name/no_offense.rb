begin
  foo
rescue => e
  bar(e)
end
begin
  foo
rescue StandardError => e
  bar(e)
end
begin
  foo
rescue
  bar
end

# Nested rescues are skipped to avoid shadowing outer variable
begin
  something
rescue LoadError => e
  raise if e.path != target
  begin
    something_else
  rescue LoadError => error_for_namespaced_target
    raise error_for_namespaced_target
  end
end

# Shadowed: preferred name assigned as lvar in body
begin
  do_something
rescue StandardError => err
  e = err.cause
  log(e)
end

# Shadowed: nested rescue uses preferred name
begin
  do_something
rescue StandardError => ex
  begin
    retry_something
  rescue => e
    log(e)
  end
end

# Shadowed: preferred name used as lvar read in body
e = 'error message'
begin
  something
rescue StandardError => e1
  log(e, e1)
end
