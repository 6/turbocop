has_many :items, class_name: Item
                             ^^^^ Rails/ReflectionClassName: Use a string value for `class_name`.
belongs_to :author, class_name: User
                                ^^^^ Rails/ReflectionClassName: Use a string value for `class_name`.
has_one :profile, class_name: UserProfile.name
                              ^^^^^^^^^^^^^^^^ Rails/ReflectionClassName: Use a string value for `class_name`.
has_and_belongs_to_many :tags, class_name: Tag
                                           ^^^ Rails/ReflectionClassName: Use a string value for `class_name`.
has_many :accounts, class_name: Account.to_s
                                ^^^^^^^^^^^^ Rails/ReflectionClassName: Use a string value for `class_name`.
belongs_to :account, class_name: Foo::Bar
                                 ^^^^^^^^ Rails/ReflectionClassName: Use a string value for `class_name`.
