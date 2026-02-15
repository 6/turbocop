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
