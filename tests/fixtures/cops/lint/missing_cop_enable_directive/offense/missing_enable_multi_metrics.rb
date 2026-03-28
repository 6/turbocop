class ResourceForm
  # Forms should be initialized with an explicit +resource:+ parameter to
  # match indexers.
  # rubocop:disable Metrics/MethodLength, Metrics/AbcSize, Metrics/CyclomaticComplexity, Metrics/PerceivedComplexity
  ^ Lint/MissingCopEnableDirective: Re-enable Metrics/MethodLength cop with `# rubocop:enable` after disabling it.
  def initialize(deprecated_resource = nil, resource: nil)
    r = resource || deprecated_resource
  end # rubocop:enable Metrics/MethodLength, Metrics/AbcSize, Metrics/CyclomaticComplexity, Metrics/PerceivedComplexity
end
