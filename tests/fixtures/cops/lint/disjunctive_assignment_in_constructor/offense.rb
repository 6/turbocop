class Foo
  def initialize
    @x ||= 1
       ^^^ Lint/DisjunctiveAssignmentInConstructor: Unnecessary disjunctive assignment. Use plain assignment.
  end
end

class Bar
  def initialize
    @a ||= []
       ^^^ Lint/DisjunctiveAssignmentInConstructor: Unnecessary disjunctive assignment. Use plain assignment.
    @b ||= {}
       ^^^ Lint/DisjunctiveAssignmentInConstructor: Unnecessary disjunctive assignment. Use plain assignment.
  end
end

# Leading ||= followed by non-||= should still flag the leading ones
class Baz
  def initialize
    @delicious ||= true
               ^^^ Lint/DisjunctiveAssignmentInConstructor: Unnecessary disjunctive assignment. Use plain assignment.
    super
  end
end
