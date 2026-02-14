begin
  do_something
rescue
^^^^^^ Lint/SuppressedException: Do not suppress exceptions.
end
begin
  work
rescue
^^^^^^ Lint/SuppressedException: Do not suppress exceptions.
end
begin
  other
rescue
^^^^^^ Lint/SuppressedException: Do not suppress exceptions.
end
