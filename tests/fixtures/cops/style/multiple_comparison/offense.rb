a = "test"
a == "x" || a == "y"
^^^^^^^^^^^^^^^^^^^^ Style/MultipleComparison: Avoid comparing a variable with multiple items in a conditional, use `Array#include?` instead.

a == "x" || a == "y" || a == "z"
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MultipleComparison: Avoid comparing a variable with multiple items in a conditional, use `Array#include?` instead.

"x" == a || "y" == a
^^^^^^^^^^^^^^^^^^^^ Style/MultipleComparison: Avoid comparing a variable with multiple items in a conditional, use `Array#include?` instead.

# With method call comparisons mixed in (method calls are skipped but non-method comparisons count)
a == foo.bar || a == 'x' || a == 'y'
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MultipleComparison: Avoid comparing a variable with multiple items in a conditional, use `Array#include?` instead.

# Method call as the "variable" being compared
name == :invalid_client || name == :unauthorized_client
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MultipleComparison: Avoid comparing a variable with multiple items in a conditional, use `Array#include?` instead.

foo.bar == "a" || foo.bar == "b"
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MultipleComparison: Avoid comparing a variable with multiple items in a conditional, use `Array#include?` instead.

# Nested inside && within a larger || — each parenthesized || group is independent
if (height > width && (rotation == 0 || rotation == 180)) || (height < width && (rotation == 90 || rotation == 270))
                       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MultipleComparison: Avoid comparing a variable with multiple items in a conditional, use `Array#include?` instead.
                                                                                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MultipleComparison: Avoid comparing a variable with multiple items in a conditional, use `Array#include?` instead.
end

# Method call as variable compared against symbol literals
x.flag == :> || x.flag == :>=
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MultipleComparison: Avoid comparing a variable with multiple items in a conditional, use `Array#include?` instead.

# Hash-like access as variable compared against local variables
outer_left_x = 48.24
outer_right_x = 547.04
lines.select { |it| it[:from][:x] == outer_left_x || it[:from][:x] == outer_right_x }
                    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MultipleComparison: Avoid comparing a variable with multiple items in a conditional, use `Array#include?` instead.

# Mixed comparison chain: flag the repeated local comparisons before the different receiver.
def determine_peticion_type(data)
  three_ds_info = data.dig(:three_ds_data, :threeDSInfo)
  return 'trataPeticion' if three_ds_info == 'AuthenticationData' ||
                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MultipleComparison: Avoid comparing a variable with multiple items in a conditional, use `Array#include?` instead.
                            three_ds_info == 'ChallengeResponse' ||
                            data[:sca_exemption] == 'MIT'
end

# `or` chains behave like `||` and still only flag the first repeated variable group.
bsopt = :disabled
ofmt = :nhx
unless bsopt == :disabled or bsopt == :molphy or ofmt == :nhx or ofmt == :molphy
       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MultipleComparison: Avoid comparing a variable with multiple items in a conditional, use `Array#include?` instead.
end

# Distinct receiver subchains in the same || tree still flag the first repeated group.
def find_fragment(attrs, data_fragments)
  data_fragments.detect do |fragment|
    !attrs.map do |k, v|
      safe_v = v.to_s
      safe_k = k.to_s
      fragment[k] == v ||
      ^^^^^^^^^^^^^^^^^^^ Style/MultipleComparison: Avoid comparing a variable with multiple items in a conditional, use `Array#include?` instead.
        fragment[k] == safe_v ||
        fragment[safe_k] == v ||
        fragment[safe_k] == safe_v
    end.include?(false)
  end
end

# A later method-call comparison should not suppress the earlier repeated local comparison group.
Dir.entries(root).each do |language|
  next if language == '.' || language == '..' || language == 'Binary' ||
          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MultipleComparison: Avoid comparing a variable with multiple items in a conditional, use `Array#include?` instead.
          File.basename(language) == 'ace_modes.json'
end

# Parenthesized groups inside && still detect the inner repeated comparison chain.
r = []
name = 'name'
type = 'type'
r[0] == name && (r[1] == type || r[1] == '*' || type == '*')
                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MultipleComparison: Avoid comparing a variable with multiple items in a conditional, use `Array#include?` instead.
