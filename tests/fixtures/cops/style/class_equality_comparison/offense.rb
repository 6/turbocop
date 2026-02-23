var.class == Date
    ^^^^^ Style/ClassEqualityComparison: Use `instance_of?` instead of comparing classes.

var.class.equal?(Date)
    ^^^^^ Style/ClassEqualityComparison: Use `instance_of?` instead of comparing classes.

var.class.eql?(Date)
    ^^^^^ Style/ClassEqualityComparison: Use `instance_of?` instead of comparing classes.

var.class.name == 'Date'
          ^^^^ Style/ClassEqualityComparison: Use `instance_of?` instead of comparing classes.
