User
^^^^ Lint/ConstantResolution: Fully qualify this constant to avoid possibly ambiguous resolution.
Login
^^^^^ Lint/ConstantResolution: Fully qualify this constant to avoid possibly ambiguous resolution.
Config
^^^^^^ Lint/ConstantResolution: Fully qualify this constant to avoid possibly ambiguous resolution.
# Unqualified root of a qualified superclass IS flagged (RuboCop does this too)
class MyService < Base::Service
                  ^^^^ Lint/ConstantResolution: Fully qualify this constant to avoid possibly ambiguous resolution.
end
# ConstantPathWriteNode: the root constant IS flagged (not a module definition)
Config::Setting = 42
^^^^^^ Lint/ConstantResolution: Fully qualify this constant to avoid possibly ambiguous resolution.
Namespace::SubConst = "value"
^^^^^^^^^ Lint/ConstantResolution: Fully qualify this constant to avoid possibly ambiguous resolution.
Parent::Child = [1, 2, 3]
^^^^^^ Lint/ConstantResolution: Fully qualify this constant to avoid possibly ambiguous resolution.
