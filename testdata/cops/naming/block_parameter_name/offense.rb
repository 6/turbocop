arr.each { |Foo| Foo }
            ^^^ Naming/BlockParameterName: Block parameter must not contain capital letters.
arr.map { |Bar| Bar }
           ^^^ Naming/BlockParameterName: Block parameter must not contain capital letters.
arr.each { |camelCase| camelCase }
            ^^^^^^^^^ Naming/BlockParameterName: Block parameter must not contain capital letters.
