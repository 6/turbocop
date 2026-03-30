around do
^^^^^^^^^ RSpec/AroundBlock: Test object should be passed to around block.
  do_something
end

around(:each) do
^^^^^^^^^^^^^^^^ RSpec/AroundBlock: Test object should be passed to around block.
  do_something
end

around do |test|
           ^^^^ RSpec/AroundBlock: You should call `test.call` or `test.run`.
  do_something
end

around(:each) do |example|
                  ^^^^^^^ RSpec/AroundBlock: You should call `example.call` or `example.run`.
  example.run(test_server)
end

around :exec do |proxy, &block|
                 ^^^^^ RSpec/AroundBlock: You should call `proxy.call` or `proxy.run`.
  values << 'before'
  result = proxy.call(&block)
  values << 'after'
  result
end
