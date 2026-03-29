collection = {}
collection [index]
          ^ Layout/SpaceBeforeBrackets: Remove the space before the opening brackets.
@hash [key]
     ^ Layout/SpaceBeforeBrackets: Remove the space before the opening brackets.
arr = []
arr [0]
   ^ Layout/SpaceBeforeBrackets: Remove the space before the opening brackets.
@correction [index_or_key] = :value
           ^ Layout/SpaceBeforeBrackets: Remove the space before the opening brackets.
collection.call(arg) [index]
                    ^ Layout/SpaceBeforeBrackets: Remove the space before the opening brackets.
value = nil
value [0] += 1
     ^ Layout/SpaceBeforeBrackets: Remove the space before the opening brackets.
value = nil
value [
     ^ Layout/SpaceBeforeBrackets: Remove the space before the opening brackets.
  0
] += 1

expect(description_hash_without_package_system["patterns"]["_attributes"] \
                                                                         ^ Layout/SpaceBeforeBrackets: Remove the space before the opening brackets.
  ["patterns_system"]).to eq("zypper")
