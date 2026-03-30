around do |test|
  test.run
end
around do |test|
  test.call
end
around do |test|
  1.times(&test)
end
around do |test|
  something_that_might_run_test(test, another_arg)
end
# yield is a valid way to run the example
around do
  setup_something
  yield
  teardown_something
end
around do |_example|
  yield
end
# example.run inside rescue/ensure blocks
around do |example|
  example.run
rescue StandardError
  handle_error
ensure
  cleanup
end
# example.run inside nested block with ensure
around do |example|
  Timeout.timeout(30) do
    example.run
  rescue TimeoutError
    retry
  ensure
    cleanup
  end
end
# example.run inside a variable assignment
around do |example|
  measurement = Benchmark.measure { example.run }
  log(measurement)
end
# example.run inside if/else branches
around do |example|
  next example.run if example.metadata[:skip_wrapper]

  if some_condition
    wrapper.perform do
      example.run
    end
  else
    other_wrapper.wrap do
      example.run
    end
  end
end
# example.run inside case/when
around do |example|
  case env_type
  when :production
    example.run
  when :staging
    example.run
  end
end
# example.run with boolean operators
around do |example|
  setup && example.run
end
# example passed to method inside deeply nested structure
around do |example|
  retries = 3
  begin
    example.run
  rescue Timeout::Error
    retries -= 1
    retry if retries > 0
  end
end
# around with a receiver should not be flagged (not an RSpec hook)
config.around(:each) do |example|
  do_something
end
c.around do
  do_something
end
obj.around do |test|
  test.inspect
end
# Block arg style |&block| with block.call is valid
around do |&block|
  super() do
    block.call
  end
end
around :exec do |proxy|
  proxy.call
end
around :exec do |proxy|
  proxy.run
end
# around with multiple args beyond sym doesn't match hook_block pattern
around(:each, net_connect: true) do |ex|
  ex.run_with_retry retry: 2
end
