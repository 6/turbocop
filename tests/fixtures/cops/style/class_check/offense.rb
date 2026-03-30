var.kind_of?(Date)
    ^^^^^^^^ Style/ClassCheck: Prefer `Object#is_a?` over `Object#kind_of?`.

var.kind_of?(Integer)
    ^^^^^^^^ Style/ClassCheck: Prefer `Object#is_a?` over `Object#kind_of?`.

var.kind_of?(String)
    ^^^^^^^^ Style/ClassCheck: Prefer `Object#is_a?` over `Object#kind_of?`.

if kind_of?(ExtManagementSystem)
   ^^^^^^^^ Style/ClassCheck: Prefer `Object#is_a?` over `Object#kind_of?`.

base.nil? || kind_of?(Vm) ? base : [base - Metric::ConfigSettings.send(:"host_overhead_#{info[:overhead_type]}"), 0.0].max
             ^^^^^^^^ Style/ClassCheck: Prefer `Object#is_a?` over `Object#kind_of?`.

records = kind_of?(Class) ? all : self
          ^^^^^^^^ Style/ClassCheck: Prefer `Object#is_a?` over `Object#kind_of?`.

if kind_of?(ManageIQ::Providers::Openstack::InfraManager) && value[:auth_key]
   ^^^^^^^^ Style/ClassCheck: Prefer `Object#is_a?` over `Object#kind_of?`.

self.type = corresponding_model.name if (template? && kind_of?(Vm)) || (!template? && kind_of?(MiqTemplate))
                                                      ^^^^^^^^ Style/ClassCheck: Prefer `Object#is_a?` over `Object#kind_of?`.
                                                                                      ^^^^^^^^ Style/ClassCheck: Prefer `Object#is_a?` over `Object#kind_of?`.

raise ArgumentError unless kind_of? other.class
                           ^^^^^^^^ Style/ClassCheck: Prefer `Object#is_a?` over `Object#kind_of?`.

raise ArgumentError unless kind_of? other.class
                           ^^^^^^^^ Style/ClassCheck: Prefer `Object#is_a?` over `Object#kind_of?`.

raise ArgumentError unless kind_of? other.class
                           ^^^^^^^^ Style/ClassCheck: Prefer `Object#is_a?` over `Object#kind_of?`.
