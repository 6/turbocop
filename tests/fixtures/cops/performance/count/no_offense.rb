[1, 2, 3].count { |x| x > 1 }
[1, 2, 3].count
arr.count
[1, 2, 3].map { |e| e + 1 }.size
[1, 2, 3].size
# select with string arg (Active Record)
Model.select('field AS field_one').count
# select with symbol arg (Active Record)
Model.select(:value).count
# interstitial method between select and count
array.select(&:value).uniq.count
# bang methods are not flagged
[1, 2, 3].select! { |e| e.odd? }.size
[1, 2, 3].reject! { |e| e.odd? }.count
# select...count with a block on count (allowed)
[1, 2, 3].select { |e| e.odd? }.count { |e| e > 2 }
