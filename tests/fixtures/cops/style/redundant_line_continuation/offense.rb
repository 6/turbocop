foo(1, \
       ^ Style/RedundantLineContinuation: Redundant line continuation.
  2)

x = 1 + \
        ^ Style/RedundantLineContinuation: Redundant line continuation.
  2

[1, \
    ^ Style/RedundantLineContinuation: Redundant line continuation.
 2]

if children \
            ^ Style/RedundantLineContinuation: Redundant line continuation.
  .reject { |c| c }
end

obj.elements['BuildAction'] \
                            ^ Style/RedundantLineContinuation: Redundant line continuation.
  .elements['Next']

foo(bar) \
         ^ Style/RedundantLineContinuation: Redundant line continuation.
  .baz

foo \
    ^ Style/RedundantLineContinuation: Redundant line continuation.
  .bar \
       ^ Style/RedundantLineContinuation: Redundant line continuation.
    .baz

foo&. \
      ^ Style/RedundantLineContinuation: Redundant line continuation.
  bar

foo do \
       ^ Style/RedundantLineContinuation: Redundant line continuation.
  bar
end

class Foo \
          ^ Style/RedundantLineContinuation: Redundant line continuation.
end

foo \
    ^ Style/RedundantLineContinuation: Redundant line continuation.
  && bar

foo \
    ^ Style/RedundantLineContinuation: Redundant line continuation.
  || bar

def merge_strategy(namespace_subclasses)
  return nil unless namespace_subclasses.empty? \
                                                ^ Style/RedundantLineContinuation: Redundant line continuation.
    || (namespace_subclasses.length == 1 && namespace_subclasses.first < RbiGenerator::Namespace) \
                                                                                                  ^ Style/RedundantLineContinuation: Redundant line continuation.
    || namespace_subclasses.to_set == Set[RbiGenerator::ClassNamespace, RbiGenerator::StructClassNamespace] \
                                                                                                            ^ Style/RedundantLineContinuation: Redundant line continuation.
    || namespace_subclasses.to_set == Set[RbiGenerator::ClassNamespace, RbiGenerator::EnumClassNamespace]
end

(name.nil? ? true : child.name == name) \
                                        ^ Style/RedundantLineContinuation: Redundant line continuation.
  && (type.nil? ? true : child.is_a?(type))

Constant === other && name == other.name && value == other.value \
                                                                 ^ Style/RedundantLineContinuation: Redundant line continuation.
  && eigen_constant == other.eigen_constant && heredocs == other.heredocs

paths.each do |path|
  next if !expanded_inclusions.any? { |i| path.start_with?(i) } \
                                                                ^ Style/RedundantLineContinuation: Redundant line continuation.
    || expanded_exclusions.any? { |e| path.start_with?(e) }
end

parse_err 'node after a sig must be a method definition', def_node \
  unless [:attr_reader, :attr_writer, :attr_accessor].include?(method_name) \
                                                                            ^ Style/RedundantLineContinuation: Redundant line continuation.
    || target != nil

(! items.empty?) or \
                    ^ Style/RedundantLineContinuation: Redundant line continuation.
  raise("error")

(arity == req_arity) or \
                        ^ Style/RedundantLineContinuation: Redundant line continuation.
  raise ArgumentError, "invalid"

valid && other and \
                   ^ Style/RedundantLineContinuation: Redundant line continuation.
  do_something

errors << "required" if \
                        ^ Style/RedundantLineContinuation: Redundant line continuation.
  config.nil?

raise "error" unless \
                     ^ Style/RedundantLineContinuation: Redundant line continuation.
  valid?

refs = (cond \
  ? self.refs \
              ^ Style/RedundantLineContinuation: Redundant line continuation.
  : other_attrs)

@table = \
         ^ Style/RedundantLineContinuation: Redundant line continuation.
  find_table || default_table
