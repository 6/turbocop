enum status: { active: 0, inactive: 0 }
                                    ^ Rails/EnumUniqueness: Duplicate value `0` found in `status` enum declaration.

enum :role, { admin: 1, user: 1 }
                              ^ Rails/EnumUniqueness: Duplicate value `1` found in `role` enum declaration.

enum priority: { low: 0, medium: 1, high: 0 }
                                          ^ Rails/EnumUniqueness: Duplicate value `0` found in `priority` enum declaration.
