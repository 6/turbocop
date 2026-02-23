0.times { it }
          ^^ Lint/ItWithoutArgumentsInBlock: `it` calls without arguments will refer to the first block param in Ruby 3.4; use `it()` or `self.it`.
do_something { it }
               ^^ Lint/ItWithoutArgumentsInBlock: `it` calls without arguments will refer to the first block param in Ruby 3.4; use `it()` or `self.it`.
foo.each { it }
           ^^ Lint/ItWithoutArgumentsInBlock: `it` calls without arguments will refer to the first block param in Ruby 3.4; use `it()` or `self.it`.
