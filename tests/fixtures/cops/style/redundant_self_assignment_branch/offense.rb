# Self-assigning else branch in if/else
foo = if condition
        bar
      else
        foo
        ^^^ Style/RedundantSelfAssignmentBranch: Remove the self-assignment branch.
      end

# Self-assigning if branch in if/else
foo = if condition
        foo
        ^^^ Style/RedundantSelfAssignmentBranch: Remove the self-assignment branch.
      else
        bar
      end

# Self-assigning else branch with empty if branch
foo = if condition
      else
        foo
        ^^^ Style/RedundantSelfAssignmentBranch: Remove the self-assignment branch.
      end

# Self-assigning if branch with empty else branch
foo = if condition
        foo
        ^^^ Style/RedundantSelfAssignmentBranch: Remove the self-assignment branch.
      else
      end

t = t.is_a?(String) ? t : t.describe
                      ^ Style/RedundantSelfAssignmentBranch: Remove the self-assignment branch.

id = id.include?('#') ? id.split('#', 2).last : id
                                                ^^ Style/RedundantSelfAssignmentBranch: Remove the self-assignment branch.

regexp = regexp.is_a?(Regexp) ?
  regexp :
  ^^^^^^ Style/RedundantSelfAssignmentBranch: Remove the self-assignment branch.
  Regexp.new(regexp.to_s, Regexp::IGNORECASE)
