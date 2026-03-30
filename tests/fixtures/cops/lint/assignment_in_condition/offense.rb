if x = 1
     ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
  do_something
end

while y = gets
        ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
  process(y)
end

until z = calculate
        ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
  retry_something
end

if @test = 10
         ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
end

if @@test = 10
          ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
end

if $test = 10
         ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
end

if TEST = 10
        ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
end

if test == 10 || foobar = 1
                        ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
end

if test.method = 10
               ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
end

if test&.method = 10
                ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
end

if a[3] = 10
        ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
end

do_something if x = 1
                  ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.

do_something while y = gets
                     ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.

unless x = 1
         ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
  do_something
end

if (foo == bar && test = 10)
                       ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
end

if (foo == bar || test = 10)
                       ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
end

foo { x if y = z }
             ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.

raise StandardError unless (foo ||= bar) || a = b
                                              ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.

if Foo::Bar = 1
            ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
end

while Foo::Bar = fetch_data
               ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
end

unless Module::Config = load_config
                      ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
  apply_defaults
end

if Foo::Bar = 1 || baz
            ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
end

# Assignment inside begin..end used as part of an if condition
if valid? && begin
  result = compute_value
         ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
  result.present?
end
  use(result)
end

# Assignment inside begin..end used as while condition
while begin item = queue.shift; item end
                 ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
  process(item)
end

# Assignment in when condition (bare case used as elsif condition)
if false
elsif case
when match = scan(/foo/)
           ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
  process(match)
end
end

# Assignment inside case/when body within an if condition
if (case kind
      when :special
        found = lookup(kind)
              ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
      else
        false
    end)
  use(found)
end

# Assignment inside begin/rescue used as condition
return true if check? && begin
  data = parse(input)
       ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
  data.valid?
rescue StandardError
  false
end

# Assignment inside rescue modifier in condition
return nil unless valid? || begin
  uri = URI.parse(route) rescue nil
      ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
  uri.present?
end

# Nested assignment: value of outer assignment contains another assignment via &&
if a = foo &&
     ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
    b = bar
      ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
end

# Nested assignment inside begin block that is assignment value in condition
if klass = begin
         ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
    result_id = lookup
              ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
    result_id.present?
  rescue
  end
end

# Triple-chain: call && assignment whose value is && assignment
if check && student = find_user &&
                    ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
    offering = find_offering
             ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
end

# Assignment inside class/def body that is part of an if condition (via &&)
# The class expression is the RHS of &&, so assignments within are "in the condition"
if should_install? &&
    class RakeTest
      def test_records_trace
        trace = single_transaction_trace_posted
              ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
      end
      def test_records_events
        event = single_event_posted[0]
              ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
      end
    end
end
