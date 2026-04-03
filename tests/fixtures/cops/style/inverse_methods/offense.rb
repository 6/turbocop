!foo.any?
^^^^^^^^^ Style/InverseMethods: Use `none?` instead of inverting `any?`.
!foo.none?
^^^^^^^^^^ Style/InverseMethods: Use `any?` instead of inverting `none?`.
!foo.even?
^^^^^^^^^^ Style/InverseMethods: Use `odd?` instead of inverting `even?`.
!(x == false)
^^^^^^^^^^^^^ Style/InverseMethods: Use `!=` instead of inverting `==`.
items.select { |x| !x.valid? }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/InverseMethods: Use `reject` instead of inverting `select`.
items.reject { |k, v| v != :active }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/InverseMethods: Use `select` instead of inverting `reject`.
items.select! { |x| !x.empty? }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/InverseMethods: Use `reject!` instead of inverting `select!`.
items.reject! { |k, v| v != :a }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/InverseMethods: Use `select!` instead of inverting `reject!`.
items.reject do |x|
^^^^^^^^^^^^^^^^^^ Style/InverseMethods: Use `select` instead of inverting `reject`.
  !x.nil?
end
!(x != y)
^^^^^^^^^ Style/InverseMethods: Use `==` instead of inverting `!=`.
!(x !~ /pattern/)
^^^^^^^^^^^^^^^^^^ Style/InverseMethods: Use `=~` instead of inverting `!~`.
not foo.any?
^^^^^^^^^^^^ Style/InverseMethods: Use `none?` instead of inverting `any?`.
foo&.select { |e| !e }
^^^^^^^^^^^^^^^^^^^^^^^ Style/InverseMethods: Use `reject` instead of inverting `select`.

labels.reject! { |label| !ACTIVE_LABELS.include?(label) }
^ Style/InverseMethods: Use `select!` instead of inverting `reject!`.

h.reject! { |k| !update_only.include? k } if update_only.any?
^ Style/InverseMethods: Use `select!` instead of inverting `reject!`.

@namespace.constants.reject{ |c| !get_obj( c ).is_a?( Class ) }
^ Style/InverseMethods: Use `select` instead of inverting `reject`.

options['plugins'].reject! { |k, _| !@plugins[k].distributable? }
^ Style/InverseMethods: Use `select!` instead of inverting `reject!`.

flat.reject { |i| !i.is_a? Symbol }
^ Style/InverseMethods: Use `select` instead of inverting `reject`.

hash = env.reject{ |k, v| !k.to_s.downcase.include?( 'http' ) }.inject({}) do |h, (k, v)|
       ^ Style/InverseMethods: Use `select` instead of inverting `reject`.

expect(mutable.mutations( seed ).
       ^ Style/InverseMethods: Use `select` instead of inverting `reject`.
  reject { |e| e.affected_input_name != input }).
  to be_empty

expect(mutable.mutations( seed ).
       ^ Style/InverseMethods: Use `select` instead of inverting `reject`.
  reject { |e| e.affected_input_name != input }).
  to be_any

matching_relationships.reject! do |relationship|
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/InverseMethods: Use `select!` instead of inverting `reject!`.
  !our_columns.any? { |c| relationship[c] }
end

headers.reject { |k, _| !(k =~ /X-Object-Meta-/) }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/InverseMethods: Use `select` instead of inverting `reject`.

if !(@file.class <= IO) && !@file.instance_of?(StringIO)
   ^ Style/InverseMethods: Use `>` instead of inverting `<=`.

raise 'invalid cpu' if not cpu < CPU
                       ^^^ Style/InverseMethods: Use `>=` instead of inverting `<`.

return if !(RUBY_VERSION >= '2.8.0')
          ^ Style/InverseMethods: Use `<` instead of inverting `>=`.
