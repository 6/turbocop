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
