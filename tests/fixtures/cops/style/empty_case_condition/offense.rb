case
^^^^ Style/EmptyCaseCondition: Do not use empty `case` condition, instead use an `if` expression.
when 1 == 2
  foo
when 1 == 1
  bar
else
  baz
end

case
^^^^ Style/EmptyCaseCondition: Do not use empty `case` condition, instead use an `if` expression.
when 1 == 2
  foo
when 1 == 1
  bar
end

case
^^^^ Style/EmptyCaseCondition: Do not use empty `case` condition, instead use an `if` expression.
when 1 == 2
  foo
end

x = case
    ^^^^ Style/EmptyCaseCondition: Do not use empty `case` condition, instead use an `if` expression.
    when foo.is_a?(String)
      1
    when foo.is_a?(Array)
      2
    else
      3
    end

@result = case
          ^^^^ Style/EmptyCaseCondition: Do not use empty `case` condition, instead use an `if` expression.
          when cond_a then :a
          when cond_b then :b
          else :c
          end

impl = case
       ^^^^ Style/EmptyCaseCondition: Do not use empty `case` condition, instead use an `if` expression.
       when obj.is_a?(Class)
         obj.new
       when obj.respond_to?(:call)
         obj.call
       else
         raise "unsupported"
       end

result.merge(
  key => case
         ^^^^ Style/EmptyCaseCondition: Do not use empty `case` condition, instead use an `if` expression.
         when !value.properties.empty?
           call(value.properties)
         when !value.data["example"].nil?
           value.data["example"]
         when value.type.include?("null")
           nil
         else
           "fallback"
         end
)

result.merge(
  key => case
         ^^^^ Style/EmptyCaseCondition: Do not use empty `case` condition, instead use an `if` expression.
         when !value.properties.empty?
           call(value)
         when !value.data["example"].nil?
           value.data["example"]
         when value.type.include?("null")
           nil
         else
           "fallback"
         end
)
