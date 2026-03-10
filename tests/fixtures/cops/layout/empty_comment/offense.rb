x = 1
#
^ Layout/EmptyComment: Source code comment is empty.
y = 2

z = 3
  #
  ^ Layout/EmptyComment: Source code comment is empty.
a = 4

#
^ Layout/EmptyComment: Source code comment is empty.
b = 5

def foo #
        ^ Layout/EmptyComment: Source code comment is empty.
  something #
            ^ Layout/EmptyComment: Source code comment is empty.
end

put "/users/#{name}" #
                     ^ Layout/EmptyComment: Source code comment is empty.

record.add_child("#{other.id}") #
                                ^ Layout/EmptyComment: Source code comment is empty.
