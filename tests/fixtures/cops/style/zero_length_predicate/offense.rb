[1, 2, 3].length == 0
^^^^^^^^^^^^^^^^^^^^^ Style/ZeroLengthPredicate: Use `empty?` instead of `[1, 2, 3].length == 0`.

'foobar'.length == 0
^^^^^^^^^^^^^^^^^^^^ Style/ZeroLengthPredicate: Use `empty?` instead of `'foobar'.length == 0`.

array.size == 0
^^^^^^^^^^^^^^^ Style/ZeroLengthPredicate: Use `empty?` instead of `array.size == 0`.

hash.size > 0
^^^^^^^^^^^^^ Style/ZeroLengthPredicate: Use `!empty?` instead of `hash.size > 0`.

Post.find_all.length > 0
^^^^^^^^^^^^^^^^^^^^^^^^ Style/ZeroLengthPredicate: Use `!empty?` instead of `Post.find_all.length > 0`.

Animal.db_indexes.size > 0
^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ZeroLengthPredicate: Use `!empty?` instead of `Animal.db_indexes.size > 0`.

Object.methods.length > 0
^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ZeroLengthPredicate: Use `!empty?` instead of `Object.methods.length > 0`.

ENV['FOFA_INVALID_IP'].size > 0
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ZeroLengthPredicate: Use `!empty?` instead of `ENV['FOFA_INVALID_IP'].size > 0`.

parameters&.length == 0
^^^^^^^^^^^^^^^^^^^^^^^ Style/ZeroLengthPredicate: Use `empty?` instead of `parameters&.length == 0`.

def self.contains_type_and_title?(resources, type, title)
  !resources.select do |x|
    test_type, test_title = x[1].split(/\f/)
    test_type.casecmp(type).zero? && test_title.casecmp(title).zero?
  end.size.zero?
      ^^^^^^^^^^ Style/ZeroLengthPredicate: Use `empty?` instead of `size.zero?`.
end

if @receiving_headers
  if idata&.length&.zero?
            ^^^^^^^^^^^^^ Style/ZeroLengthPredicate: Use `empty?` instead of `length&.zero?`.
    @receiving_headers = false
  end
end

branches.each do |branch|
  if last_node&.connections&.size&.zero?
                             ^^^^^^^^^^^ Style/ZeroLengthPredicate: Use `empty?` instead of `size&.zero?`.
    end_node_connected = true
  end
end
