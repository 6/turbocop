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
begin
  do_work
rescue NotImplementedError
  fallback
rescue Errno::ESRCH
^^^^^^ Lint/SuppressedException: Do not suppress exceptions.
end
# Trailing comment on rescue line does NOT satisfy AllowComments
begin
  do_something
rescue # intentionally ignored
^^^^^^ Lint/SuppressedException: Do not suppress exceptions.
end
begin
  do_something
rescue StandardError # intentionally ignored
^^^^^^ Lint/SuppressedException: Do not suppress exceptions.
end
def perform_task
  do_work
rescue RuntimeError # skip
^^^^^^ Lint/SuppressedException: Do not suppress exceptions.
end
# Trailing rubocop:disable comment on rescue line
begin
  do_something
rescue # rubocop:disable Lint/HandleExceptions
^^^^^^ Lint/SuppressedException: Do not suppress exceptions.
end
# Trailing comment on rescue inside def
def cleanup_reader
  super
rescue Errno::ESRCH # continue if not found
^^^^^^ Lint/SuppressedException: Do not suppress exceptions.
end
# Trailing comment on rescue in multi-rescue chain
begin
  do_something
rescue Errno::ECHILD # No child processes
^^^^^^ Lint/SuppressedException: Do not suppress exceptions.
rescue NotImplementedError
^^^^^^ Lint/SuppressedException: Do not suppress exceptions.
end
