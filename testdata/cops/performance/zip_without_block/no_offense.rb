[1, 2, 3].map { |id| id }
[1, 2, 3].map { |id| [id, id] }
[1, 2, 3].select { |id| [id] }
[1, 2, 3].filter_map { |id| [id] }
[1, 2, 3].flat_map { |id| [id] }
[1, 2, 3].map { |id| [[id]] }
[1, 2, 3].map { |id| id + 1 }
[1, 2, 3].map { |e| [1] }
[1, 2, 3].map { [id] }
[1, 2, 3].map { [] }
map { |id| [id] }
[1, 2, 3].map
