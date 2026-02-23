(1..5).each { }
^^^^^^ Style/EachForSimpleLoop: Use `Integer#times` for a simple loop which iterates a fixed number of times.

(0...10).each {}
^^^^^^^^ Style/EachForSimpleLoop: Use `Integer#times` for a simple loop which iterates a fixed number of times.

(0..3).each { puts "hi" }
^^^^^^ Style/EachForSimpleLoop: Use `Integer#times` for a simple loop which iterates a fixed number of times.
