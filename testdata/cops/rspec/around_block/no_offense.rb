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
