items.each { |item| do_something(item) }
other_items.each { |item| do_something(item) }

items.each { |item| foo(item) }
do_something
items.each { |item| bar(item) }

items.each { |item| foo(item) }
items.map { |item| bar(item) }
