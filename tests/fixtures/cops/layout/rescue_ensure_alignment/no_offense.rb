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

# Properly aligned rescue inside def
def fetch(key)
  @store[key]
rescue
  raise RetrievalError
end

# Properly aligned ensure inside def
def process
  do_work
ensure
  cleanup
end

# Properly aligned rescue + ensure inside def
def transform
  compute
rescue StandardError => e
  handle(e)
ensure
  finalize
end

# Properly aligned rescue inside class
class Widget
  do_something
rescue
  handle_error
end

# Properly aligned ensure inside module
module Helpers
  initialize_stuff
ensure
  finalize
end

# Properly aligned rescue inside do-end block
items.each do |item|
  item.process
rescue StandardError
  next
end

# Properly aligned ensure inside do-end block
records.map do |rec|
  rec.save
ensure
  rec.unlock
end

# private def with rescue aligned to modifier (line start)
private def fetch_data
  do_work
rescue StandardError
  handle_error
end

# private def with ensure aligned to modifier (line start)
private def process
  compute
ensure
  cleanup
end

# protected def with rescue aligned to modifier (line start)
protected def transform
  compute
rescue
  handle
end

# Nested private def with rescue aligned to modifier
module MyModule
  class MyClass
    private def risky_operation
      perform
    rescue StandardError => e
      log(e)
    end
  end
end

# Multi-line method chain with do..end block: rescue aligns with chain start
Collection.where(active: true)
           .find_each do |item|
  item.process
rescue StandardError => e
  handle(e)
end

# Assigned do-end block: rescue aligns with assignment target (line start)
result = run_callbacks :call do
  @app.call(env)
rescue => error
  handle(error)
end

# Assigned do-end block: ensure aligns with assignment target
thread2 = Thread.new do
  barrier.wait
ensure
  cleanup
end

# Assigned do-end block with lambda: rescue aligns with assignment target
test_update = lambda do |order|
  Author.order(order).update_all("id = id + 1")
rescue ActiveRecord::ActiveRecordError
  false
end

# Instance variable assigned do-end block: rescue aligns with line start
@instance = records.map do |r|
  r.process
rescue StandardError
  nil
end

# Or-assignment do-end block: rescue aligns with line start
a ||= items.map do |_|
rescue StandardError => _
end

