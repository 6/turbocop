x = [1,
     2,
     3]
y = [:a,
     :b,
     :c]
z = [1, 2, 3]
[1,
 2]

# Multiple elements per line, first element aligned
owned_classes = [
  Status, StatusPin, MediaAttachment, Poll, Report, Tombstone, Favourite,
  Follow, FollowRequest, Block, Mute,
  AccountModerationNote, AccountPin, AccountStat, ListAccount,
  PollVote, Mention, AccountDeletionRequest, AccountNote,
  Appeal, TagFollow
]

# Element is not the first token on its line (}, { pattern)
actions = [
  {
    edit: { range: range, newText: text }
  }, {
    edit: { range: other_range, newText: other_text }
  }
]
