enum status: { active: 0, inactive: 0 }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/EnumUniqueness: Duplicate enum value `0` detected.

enum :role, { admin: 1, user: 1 }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/EnumUniqueness: Duplicate enum value `1` detected.

enum priority: { low: 0, medium: 1, high: 0 }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/EnumUniqueness: Duplicate enum value `0` detected.
