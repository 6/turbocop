# if/else identical trailing lines
if condition
  do_x
  do_z
  ^^^^ Style/IdenticalConditionalBranches: Move `do_z` out of the conditional.
else
  do_y
  do_z
  ^^^^ Style/IdenticalConditionalBranches: Move `do_z` out of the conditional.
end
if foo
  bar
  result
  ^^^^^^ Style/IdenticalConditionalBranches: Move `result` out of the conditional.
else
  baz
  result
  ^^^^^^ Style/IdenticalConditionalBranches: Move `result` out of the conditional.
end
if x
  a = 1
  b
  ^ Style/IdenticalConditionalBranches: Move `b` out of the conditional.
else
  a = 2
  b
  ^ Style/IdenticalConditionalBranches: Move `b` out of the conditional.
end

# if/else identical leading lines
if something
  do_x
  ^^^^ Style/IdenticalConditionalBranches: Move `do_x` out of the conditional.
  method_call_here(1, 2, 3)
else
  do_x
  ^^^^ Style/IdenticalConditionalBranches: Move `do_x` out of the conditional.
  1 + 2 + 3
end

# if/elsif/else identical trailing lines
if cond_a
  x1
  do_z
  ^^^^ Style/IdenticalConditionalBranches: Move `do_z` out of the conditional.
elsif cond_b
  x2
  do_z
  ^^^^ Style/IdenticalConditionalBranches: Move `do_z` out of the conditional.
else
  x3
  do_z
  ^^^^ Style/IdenticalConditionalBranches: Move `do_z` out of the conditional.
end

# if/elsif/else identical leading lines
if cond_a
  do_z
  ^^^^ Style/IdenticalConditionalBranches: Move `do_z` out of the conditional.
  x1
elsif cond_b
  do_z
  ^^^^ Style/IdenticalConditionalBranches: Move `do_z` out of the conditional.
  x2
else
  do_z
  ^^^^ Style/IdenticalConditionalBranches: Move `do_z` out of the conditional.
  x3
end

# case/when/else identical trailing lines
case something
when :a
  x1
  do_x
  ^^^^ Style/IdenticalConditionalBranches: Move `do_x` out of the conditional.
when :b
  x2
  do_x
  ^^^^ Style/IdenticalConditionalBranches: Move `do_x` out of the conditional.
else
  x3
  do_x
  ^^^^ Style/IdenticalConditionalBranches: Move `do_x` out of the conditional.
end

# case/when/else identical bodies
case something
when :a
  do_x
  ^^^^ Style/IdenticalConditionalBranches: Move `do_x` out of the conditional.
when :b
  do_x
  ^^^^ Style/IdenticalConditionalBranches: Move `do_x` out of the conditional.
else
  do_x
  ^^^^ Style/IdenticalConditionalBranches: Move `do_x` out of the conditional.
end

# case/when/else identical leading lines
case something
when :a
  do_x
  ^^^^ Style/IdenticalConditionalBranches: Move `do_x` out of the conditional.
  x1
when :b
  do_x
  ^^^^ Style/IdenticalConditionalBranches: Move `do_x` out of the conditional.
  x2
else
  do_x
  ^^^^ Style/IdenticalConditionalBranches: Move `do_x` out of the conditional.
  x3
end

# case/in/else (pattern matching) identical trailing lines
case something
in :a
  x1
  do_x
  ^^^^ Style/IdenticalConditionalBranches: Move `do_x` out of the conditional.
in :b
  x2
  do_x
  ^^^^ Style/IdenticalConditionalBranches: Move `do_x` out of the conditional.
else
  x3
  do_x
  ^^^^ Style/IdenticalConditionalBranches: Move `do_x` out of the conditional.
end

# if/else identical bodies (both head and tail — report tail)
if something
  do_x
  ^^^^ Style/IdenticalConditionalBranches: Move `do_x` out of the conditional.
else
  do_x
  ^^^^ Style/IdenticalConditionalBranches: Move `do_x` out of the conditional.
end

# if/else with identical trailing lines and assign to condition value
if x.condition
  foo
  x = do_something
  ^^^^^^^^^^^^^^^^ Style/IdenticalConditionalBranches: Move `x = do_something` out of the conditional.
else
  bar
  x = do_something
  ^^^^^^^^^^^^^^^^ Style/IdenticalConditionalBranches: Move `x = do_something` out of the conditional.
end

# if/else identical leading lines with different formatting
if RSpec::Core::Version::STRING >= '3'
  c.include Ammeter::RSpec::Rails::GeneratorExampleHelpers, :type          => :generator
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/IdenticalConditionalBranches: Move `c.include Ammeter::RSpec::Rails::GeneratorExampleHelpers, :type          => :generator` out of the conditional.
  c.include Ammeter::RSpec::Rails::GeneratorExampleGroup,
    :type          => :generator
else
  c.include Ammeter::RSpec::Rails::GeneratorExampleHelpers, :type => :generator
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/IdenticalConditionalBranches: Move `c.include Ammeter::RSpec::Rails::GeneratorExampleHelpers, :type          => :generator` out of the conditional.
  c.include Ammeter::RSpec::Rails::GeneratorExampleGroup, :type => :generator, :example_group => {
    :file_path => generator_path_regex
  }
end

# if/else identical trailing lines with different formatting
if @root_object.is_a?(Resource)
  ao_ids = archive_ids
  date_query = date_query.filter(:archival_object_id => ao_ids)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/IdenticalConditionalBranches: Move `date_query = date_query.filter(:archival_object_id => ao_ids)` out of the conditional.
else
  ao_ids = []
  date_query = date_query.filter(:archival_object_id  => ao_ids)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/IdenticalConditionalBranches: Move `date_query = date_query.filter(:archival_object_id => ao_ids)` out of the conditional.
end

# unless/else identical trailing lines
unless condition
  do_x
  do_z
  ^^^^ Style/IdenticalConditionalBranches: Move `do_z` out of the conditional.
else
  do_y
  do_z
  ^^^^ Style/IdenticalConditionalBranches: Move `do_z` out of the conditional.
end

# unless/else identical leading lines
unless something
  do_x
  ^^^^ Style/IdenticalConditionalBranches: Move `do_x` out of the conditional.
  method_call_here(1, 2, 3)
else
  do_x
  ^^^^ Style/IdenticalConditionalBranches: Move `do_x` out of the conditional.
  1 + 2 + 3
end

# unless/else identical bodies
unless condition
  do_x
  ^^^^ Style/IdenticalConditionalBranches: Move `do_x` out of the conditional.
else
  do_x
  ^^^^ Style/IdenticalConditionalBranches: Move `do_x` out of the conditional.
end

# unless/else identical trailing lines (method call with bang)
unless params[:collection_id].blank?
  work.collection = @collection
  work.save!
  ^^^^^^^^^^ Style/IdenticalConditionalBranches: Move `work.save!` out of the conditional.
else
  collection = Collection.new
  work.save!
  ^^^^^^^^^^ Style/IdenticalConditionalBranches: Move `work.save!` out of the conditional.
end

# unless/else identical bodies (return statements)
unless (defined? @ipr_ids) && @ipr_ids
  @ipr_ids = {}
  return @ipr_ids
  ^^^^^^^^^^^^^^^ Style/IdenticalConditionalBranches: Move `return @ipr_ids` out of the conditional.
else
  return @ipr_ids
  ^^^^^^^^^^^^^^^ Style/IdenticalConditionalBranches: Move `return @ipr_ids` out of the conditional.
end
