arr.each { |Foo| Foo }
            ^^^ Naming/BlockParameterName: Block parameter must not contain capital letters.
arr.map { |Bar| Bar }
           ^^^ Naming/BlockParameterName: Block parameter must not contain capital letters.
arr.each { |camelCase| camelCase }
            ^^^^^^^^^ Naming/BlockParameterName: Block parameter must not contain capital letters.
foo { |__| bar }
       ^^ Naming/BlockParameterName: Block parameter name is too short.
foo { |___| bar }
       ^^^ Naming/BlockParameterName: Block parameter name is too short.
foo { |____| bar }
       ^^^^ Naming/BlockParameterName: Block parameter name is too short.
foo { |FOO:| bar }
       ^^^ Naming/BlockParameterName: Block parameter must not contain capital letters.
foo { |bar:, BAZ: nil| bar }
             ^^^ Naming/BlockParameterName: Block parameter must not contain capital letters.
# Lambda parameters should also be checked
->(locationID) { locationID }
   ^^^^^^^^^^ Naming/BlockParameterName: Block parameter must not contain capital letters.
-> (Foo, Bar) { [Foo, Bar] }
    ^^^ Naming/BlockParameterName: Block parameter must not contain capital letters.
         ^^^ Naming/BlockParameterName: Block parameter must not contain capital letters.
