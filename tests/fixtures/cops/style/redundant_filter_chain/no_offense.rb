arr.any? { |x| x > 1 }
arr.none? { |x| x > 1 }
arr.select { |x| x > 1 }.count
arr.select(:name).any?
foo.select.any?
arr.select { |x| x > 1 }.any?(Integer)
