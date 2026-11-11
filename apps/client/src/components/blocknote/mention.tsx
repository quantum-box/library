import { createReactInlineContentSpec } from '@blocknote/react'

export const userMention = createReactInlineContentSpec(
  {
    type: 'userMention',
    propSchema: { userId: { default: '' }, name: { default: '' } },
    content: 'none',
  },
  {
    render: ({ inlineContent }) => (
      <span className="rounded bg-primary/10 px-1 text-primary" data-user-id={inlineContent.props.userId}>
        @{inlineContent.props.name}
      </span>
    ),
    toExternalHTML: ({ inlineContent }) => <span>@{inlineContent.props.name}</span>,
  },
)
