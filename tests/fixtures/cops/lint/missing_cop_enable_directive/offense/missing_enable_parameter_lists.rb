class NestCollectionForm
  # @param context [#can?,#repository,#blacklight_config]
  # rubocop:disable Metrics/ParameterLists
  ^ Lint/MissingCopEnableDirective: Re-enable Metrics/ParameterLists cop with `# rubocop:enable` after disabling it.
  def initialize(parent: nil,
                 child: nil,
                 parent_id: nil,
                 child_id: nil,
                 context:,
                 query_service: default_query_service,
                 persistence_service: default_persistence_service)
  end # rubocop:enable Metrics/ParameterLists
end
