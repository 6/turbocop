array.sort

array.sort_by { |x| x.length }

array.sort_by { |x| -x }

array.sort_by(&:name)

array.sort_by { |a, b| a }
