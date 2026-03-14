import type { Meta, StoryObj } from '@storybook/react'
import { CodeSnippet } from './code-snippet'

export default {
	title: 'V1Beta/CodeSnippet',
	component: CodeSnippet,
	parameters: {
		test: {
			dangerouslyIgnoreUnhandledErrors: true,
		},
	},
	tags: ['api-page'],
} satisfies Meta<typeof CodeSnippet>

type Story = StoryObj<typeof CodeSnippet>

export const Default: Story = {
	args: {
		snippets: {
			curl: `curl -X GET "https://api.example.com/v1beta/repos/quanta/book/data-list" \\
  -H "Authorization: Bearer pk_YOUR_API_KEY"`,
			python: `import requests

response = requests.get(
    "https://api.example.com/v1beta/repos/quanta/book/data-list",
    headers={"Authorization": "Bearer pk_YOUR_API_KEY"}
)
print(response.json())`,
			javascript: `const response = await fetch(
  "https://api.example.com/v1beta/repos/quanta/book/data-list",
  {
    headers: { "Authorization": "Bearer pk_YOUR_API_KEY" },
  }
);
const data = await response.json();
console.log(data);`,
		},
	},
}
