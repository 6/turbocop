[2, 1, 3].min
[2, 1, 3].max
[2, 1, 3].sort
[2, 1, 3].sort[1]
[2, 1, 3].sort.reverse
arr.min_by(&:foo)
[1, 2, 3].sort.first(1)
[1, 2, 3].sort_by.first
mongo_client["users"].find.sort(_id: 1).first
[1, 2, 3].sort!.first
[1, 2, 3].sort_by!(&:something).last
[[1, 2], [3, 4]].first.sort
[1, 2, 3].sort_by(&:foo).at(-2)
mongo_client["users"].find.sort(_id: 1)[-1]
languages.sort(&method(:version_compare)).last
languages.sort(&:foo).first
