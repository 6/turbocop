hash.merge!(a: 1)
^^^^^^^^^^^^^^^^^ Performance/RedundantMerge: Use `[]=` instead of `merge!` with a single key-value pair.
hash.merge!(key: value)
^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantMerge: Use `[]=` instead of `merge!` with a single key-value pair.
opts.merge!(debug: true)
^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantMerge: Use `[]=` instead of `merge!` with a single key-value pair.
h = {}
h.merge!(a: 1, b: 2)
^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantMerge: Use `[]=` instead of `merge!` with 2 key-value pairs.
puts "done"
settings = {}
settings.merge!(Port: port, Host: bind)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantMerge: Use `[]=` instead of `merge!` with 2 key-value pairs.
start_server
jar = cookies('foo=bar')
jar.merge! :bar => 'baz'
^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantMerge: Use `[]=` instead of `merge!` with a single key-value pair.
expect(jar).to include('bar')
# merge! inside begin/rescue — value not used at top level
begin
  h.merge!(a: 1)
  ^^^^^^^^^^^^^^^ Performance/RedundantMerge: Use `[]=` instead of `merge!` with a single key-value pair.
rescue StandardError => e
  handle(e)
end
# instance variable receiver — pure, should be flagged
@params.merge!(a: 1)
^^^^^^^^^^^^^^^^^^^^ Performance/RedundantMerge: Use `[]=` instead of `merge!` with a single key-value pair.
# class variable receiver — pure, should be flagged
@@defaults.merge!(key: value)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantMerge: Use `[]=` instead of `merge!` with a single key-value pair.
# constant receiver — pure, should be flagged
DEFAULTS.merge!(key: value)
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantMerge: Use `[]=` instead of `merge!` with a single key-value pair.
# ivar receiver with multiple pairs
@params.merge!(a: 1, b: 2)
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantMerge: Use `[]=` instead of `merge!` with 2 key-value pairs.
# self receiver — pure, should be flagged
self.merge!(key: value)
^^^^^^^^^^^^^^^^^^^^^^^ Performance/RedundantMerge: Use `[]=` instead of `merge!` with a single key-value pair.
