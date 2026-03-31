(1..100).include?(7)

something.is_a?(Array)

/pattern/ === string

x == y

x.equal?(y)

# Non-module-name constants (ALL_CAPS) are always skipped regardless of AllowOnConstant
NUMERIC_PATTERN === timezone
NAME_PATTERN === value
CONST_NAME === input

# Qualified constants with ALL_CAPS last segment are also not module names
Constants::ATOM_UNSAFE === str
Constants::PHRASE_UNSAFE === str
URI::HTTPS === @uri
Foo::Bar::ALL_CAPS === value

# Explicit .=== call with forwarded args and block — RuboCop's matcher requires
# exactly one argument child, so multi-argument calls are not matched
native.===(*args, &bl)
