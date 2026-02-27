has_many :items, class_name: "Item"
has_many :items
has_many :items, dependent: :destroy
belongs_to :user
belongs_to :user, class_name: "SpecialUser"
has_one :profile
has_one :profile, class_name: "UserProfile"
has_and_belongs_to_many :tags
has_and_belongs_to_many :tags, class_name: "CustomTag"

# Symbol values for class_name are also acceptable
has_many :associated_articles, class_name: :Article
belongs_to :root_article, class_name: :Article

# Interpolated strings are still strings
has_many :events, class_name: "Events::#{name}Event"

# .to_s calls on non-constants produce strings
has_many :events, class_name: event_klass.name.to_s
belongs_to :aggregate, class_name: klass.to_s

# Bare method calls are not constants
has_many :items, class_name: name
has_many :results, class_name: result_class
belongs_to :model, class_name: model_class

# Method calls on self are not constants
has_one :result, class_name: self.result_class
has_many :messages, class_name: self.message_class

# Method calls on non-constant receivers
has_many :accounts, class_name: some_thing.class_name
has_many :events, class_name: do_something.to_s

# Bare to_s is a method call, not a constant
has_many :accounts, class_name: to_s
