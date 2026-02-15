begin
  do_something
rescue => e
  handle(e)
end
begin
  work
rescue StandardError
  retry
end
# AllowComments: rescue with comment is allowed by default
begin
  do_something
rescue
  # Intentionally ignored
end
begin
  work
rescue
  # Expected to fail sometimes
end
