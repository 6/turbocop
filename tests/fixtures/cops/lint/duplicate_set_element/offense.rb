Set[:foo, :bar, :foo]
                ^^^^ Lint/DuplicateSetElement: Remove the duplicate element in Set.
Set.new([:foo, :bar, :foo])
                     ^^^^ Lint/DuplicateSetElement: Remove the duplicate element in Set.
[:foo, :bar, :foo].to_set
             ^^^^ Lint/DuplicateSetElement: Remove the duplicate element in Set.
