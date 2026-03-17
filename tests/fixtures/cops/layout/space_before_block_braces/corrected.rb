[1, 2].each { |x| puts x }
[1, 2].map {|x| x * 2 }
foo.select {|x| x > 1 }
-> {puts "hello"}
->() {1 + 2}
->(x) {x * 2}
expect(-> {raise "boom"}).to raise_error
