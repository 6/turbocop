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

@mti_table = \
             ^ Style/RedundantLineContinuation: Redundant line continuation.
  find_table || default_table

data = "#{params['tid']}\
                        ^ Style/RedundantLineContinuation: Redundant line continuation.
#{params['name']}\
                 ^ Style/RedundantLineContinuation: Redundant line continuation.
#{params['comment']}"

@result = \
          ^ Style/RedundantLineContinuation: Redundant line continuation.
  child_tables.find(:name, @table_name) ||
  parent_tables.find(:name, @table_name)

@_purchase ||= \
               ^ Style/RedundantLineContinuation: Redundant line continuation.
  successful_purchases.find { _1.present? } ||
  purchase_with_tax

value = \
        ^ Style/RedundantLineContinuation: Redundant line continuation.
  if condition
    "hello"
  else
    "world"
  end

@column_widths ||= \
                   ^ Style/RedundantLineContinuation: Redundant line continuation.
  all_rows.reject {|row| row.cells == :separator}.map do |row|
    row.cells.map {|cell| cell.value.length}.flatten
  end.transpose.map(&:max)

fetch('SQ') =~ \
               ^ Style/RedundantLineContinuation: Redundant line continuation.
  /(\d+) BP; (\d+) A; (\d+) C/

(a != foo \
          ^ Style/RedundantLineContinuation: Redundant line continuation.
  or b)

(a != foo \
          ^ Style/RedundantLineContinuation: Redundant line continuation.
  and b)

msg = "content #{path} from \
                            ^ Style/RedundantLineContinuation: Redundant line continuation.
#{cksum}"

=begin
foo(1, \
       ^ Style/RedundantLineContinuation: Redundant line continuation.
  2)
result \
       ^ Style/RedundantLineContinuation: Redundant line continuation.
  .to_s
x = 1 + \
        ^ Style/RedundantLineContinuation: Redundant line continuation.
  2
=end

ESCAPE = ?\\
           ^ Style/RedundantLineContinuation: Redundant line continuation.

ESCAPE_PREFIXES = %W(
  0 1 2 3 4 5 6 7 a b f n r t v \n U \\
                                      ^ Style/RedundantLineContinuation: Redundant line continuation.
).freeze

l.permit(:name, :description, :address, :latitude, :longitude,
         opening_times_attributes: \
                                   ^ Style/RedundantLineContinuation: Redundant line continuation.
         %i[day opens_at closes_at closed open_24h]).to_h

config["markdown"] = "kramdown" unless \
                                       ^ Style/RedundantLineContinuation: Redundant line continuation.
  %w(kramdown gfm commonmarkghpages).include?(config["markdown"].to_s.downcase)

should_be_integrated = if PodPrebuild.config.prebuild_job? \
                                                           ^ Style/RedundantLineContinuation: Redundant line continuation.
                       then @cache_validation.hit + @cache_validation.missed \
                                                                             ^ Style/RedundantLineContinuation: Redundant line continuation.
                       else @cache_validation.hit \
                                                  ^ Style/RedundantLineContinuation: Redundant line continuation.
                       end

origin.respond_to?(:lat) ? origin.lat \
                                      ^ Style/RedundantLineContinuation: Redundant line continuation.
                         : origin.send(:lat_column_name)

include\
       ^ Style/RedundantLineContinuation: Redundant line continuation.
  begin
    RbConfig
  rescue NameError
    Config
  end
