[1, 2, 3].reverse_each { |x| puts x }
[1, 2, 3].each { |x| puts x }
[1, 2, 3].reverse
arr.reverse.map { |x| x.to_s }
arr.reverse_each(&:process)
