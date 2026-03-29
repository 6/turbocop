foo { |a, b, | a + b }
           ^ Style/TrailingCommaInBlockArgs: Useless trailing comma present in block arguments.

baz { |item, val, | item.to_s }
                ^ Style/TrailingCommaInBlockArgs: Useless trailing comma present in block arguments.

test do |a, b,|
             ^ Style/TrailingCommaInBlockArgs: Useless trailing comma present in block arguments.
  a + b
end

lambda { |foo, bar,| do_something(foo, bar) }
                  ^ Style/TrailingCommaInBlockArgs: Useless trailing comma present in block arguments.

pairs.select { |dist,| range.include?(dist) }.tap do |_|
                    ^ Style/TrailingCommaInBlockArgs: Useless trailing comma present in block arguments.
  puts _.to_s
end.each do |dist, f1, f2|
  puts [dist, f1, f2].join(":")
end

normalize(attributes).sort_by { |name,| name }.each do |name, values|
                                     ^ Style/TrailingCommaInBlockArgs: Useless trailing comma present in block arguments.
  values.each do |value|
    puts [name, value]
  end
end

@grammar.directives.sort_by { |name,| name }.each do |name, act|
                                   ^ Style/TrailingCommaInBlockArgs: Useless trailing comma present in block arguments.
  puts [name, act]
end

@constants.sort_by { |name,| name }.map do |name, constant|
                          ^ Style/TrailingCommaInBlockArgs: Useless trailing comma present in block arguments.
  [name, constant]
end

seedable_welcome_settings
  .select { |k,| Settings::Definition[k].writable? }
              ^ Style/TrailingCommaInBlockArgs: Useless trailing comma present in block arguments.
  .each do |k, v|
    Setting[k] = v
  end

frequency_counts
  .select { |rule,| !rule.midrule? }
                 ^ Style/TrailingCommaInBlockArgs: Useless trailing comma present in block arguments.
  .sort_by { |rule, count| [-count, rule.name] }
  .each_with_index { |(rule, count), i| puts [rule, count, i] }

groups.sort_by do |day,| day end.reverse_each do |day, entries|
                      ^ Style/TrailingCommaInBlockArgs: Useless trailing comma present in block arguments.
  display(day, entries)
end

MAZEGAKI_DIC.sort_by { |key,|
                           ^ Style/TrailingCommaInBlockArgs: Useless trailing comma present in block arguments.
  key
}.each do |key, values|
  puts "#{key} /#{values.join('/')}/"
end
