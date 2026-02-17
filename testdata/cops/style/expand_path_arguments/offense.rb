File.expand_path('..', __FILE__)
     ^^^^^^^^^^^ Style/ExpandPathArguments: Use `expand_path(__dir__)` instead of `expand_path('..', __FILE__)`.

File.expand_path('../..', __FILE__)
     ^^^^^^^^^^^ Style/ExpandPathArguments: Use `expand_path('..', __dir__)` instead of `expand_path('../..', __FILE__)`.

File.expand_path('../../..', __FILE__)
     ^^^^^^^^^^^ Style/ExpandPathArguments: Use `expand_path('../..', __dir__)` instead of `expand_path('../../..', __FILE__)`.

File.expand_path('.', __FILE__)
     ^^^^^^^^^^^ Style/ExpandPathArguments: Use `expand_path(__FILE__)` instead of `expand_path('.', __FILE__)`.

File.expand_path('../../lib', __FILE__)
     ^^^^^^^^^^^ Style/ExpandPathArguments: Use `expand_path('../lib', __dir__)` instead of `expand_path('../../lib', __FILE__)`.
