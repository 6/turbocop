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
# AllowComments: rescue with comment inside def (implicit begin)
def perform
  do_work
rescue StandardError
  # Intentionally suppressed
end
def process
  do_work
rescue
  # Known to fail occasionally
end
def execute
  do_work
rescue => e
  # Log elsewhere
end
# AllowComments: trailing comment on rescue line itself
begin
  do_something
rescue # intentionally ignored
end
begin
  do_something
rescue StandardError # intentionally ignored
end
def perform_task
  do_work
rescue RuntimeError # skip
end
# Multi-rescue: empty clause followed by clause with comment in later rescue
begin
  do_something
rescue SystemExit
rescue SocketError
  handle_error
rescue Exception => e
  # handle unexpected errors
  log(e)
end
# Multi-rescue: empty clause followed by clause with comment
begin
  do_something
rescue ConnectionError
rescue DatabaseError
  # expected during setup
end
# Multi-rescue: empty clause followed by clause with comment and body
begin
  File.unlink(path)
rescue Errno::ENOENT
rescue Errno::EACCES
  # may not be able to unlink on Windows; just ignore
  return
end
# Empty rescue followed by ensure with comments
begin
  do_something
rescue Timeout::Error
ensure
  # periodically clean out resources
  cleanup
end
# Empty rescue followed by else clause with comment
begin
  require 'optional_gem'
rescue LoadError
else
  # gem loaded successfully, define helper
  def use_gem
    do_something
  end
end
