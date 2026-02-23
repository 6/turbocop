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
