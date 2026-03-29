[1, 2, 3].shuffle.first
          ^ Style/Sample: Use `sample` instead of `shuffle.first`.

[1, 2, 3].shuffle.last
          ^ Style/Sample: Use `sample` instead of `shuffle.last`.

arr.shuffle.first
    ^ Style/Sample: Use `sample` instead of `shuffle.first`.

arr.shuffle[0]
    ^ Style/Sample: Use `sample` instead of `shuffle[0]`.

arr.shuffle[-1]
    ^ Style/Sample: Use `sample` instead of `shuffle[-1]`.

arr.shuffle.at(0)
    ^ Style/Sample: Use `sample` instead of `shuffle.at(0)`.

arr.shuffle.at(-1)
    ^ Style/Sample: Use `sample` instead of `shuffle.at(-1)`.

arr.shuffle.slice(0)
    ^ Style/Sample: Use `sample` instead of `shuffle.slice(0)`.

arr.shuffle.slice(-1)
    ^ Style/Sample: Use `sample` instead of `shuffle.slice(-1)`.

arr.shuffle[0, 3]
    ^ Style/Sample: Use `sample(3)` instead of `shuffle[0, 3]`.

arr.shuffle[0..3]
    ^ Style/Sample: Use `sample(4)` instead of `shuffle[0..3]`.

arr.shuffle[0...3]
    ^ Style/Sample: Use `sample(3)` instead of `shuffle[0...3]`.

arr.shuffle.slice(0, 3)
    ^ Style/Sample: Use `sample(3)` instead of `shuffle.slice(0, 3)`.

arr.shuffle.slice(0..3)
    ^ Style/Sample: Use `sample(4)` instead of `shuffle.slice(0..3)`.

arr.shuffle(random: Random.new)[0..3]
    ^ Style/Sample: Use `sample(4, random: Random.new)` instead of `shuffle(random: Random.new)[0..3]`.

users.select do |user|
  user.connected_at
end.shuffle.first
    ^ Style/Sample: Use `sample` instead of `shuffle.first`.
