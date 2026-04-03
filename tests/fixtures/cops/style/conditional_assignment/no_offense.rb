x = if condition
  1
else
  2
end

if condition
  x = 1
else
  y = 2
end

if condition
  do_something
else
  do_other_thing
end

# elsif branches should not be flagged even if they look like simple if/else
if condition_a
  x = 1
elsif condition_b
  x = 2
else
  x = 3
end

# case without else should not be flagged
case x
when 1
  y = 1
when 2
  y = 2
end

# case where branches assign different variables
case x
when 1
  y = 1
when 2
  z = 2
else
  w = 3
end

# case where branches have different assignment types
case x
when 1
  y = 1
else
  do_something
end

# if/else with correction exceeding line length should not be flagged
if ActionView::Base.respond_to?(:with_empty_template_cache) && ActionView::Base.respond_to?(:with_view_paths)
  @apipie_renderer = ActionView::Base.with_empty_template_cache.with_view_paths(base_paths + layouts_paths)
else
  @apipie_renderer = ActionView::Base.new(base_paths + layouts_paths)
end
