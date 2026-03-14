export type HttpMethod = 'GET' | 'POST' | 'PUT' | 'DELETE'

export interface EndpointSnippets {
	curl: string
	python: string
	javascript: string
}

export interface Endpoint {
	method: HttpMethod
	path: string
	description: string
	getSnippets: (params: {
		apiBaseUrl: string
		org: string
		repo: string
	}) => EndpointSnippets
}

export interface EndpointCategory {
	key: string
	endpoints: Endpoint[]
}

export const getEndpointCategories = (): EndpointCategory[] => [
	{
		key: 'data',
		endpoints: [
			{
				method: 'GET',
				path: '/v1beta/repos/{org}/{repo}/data-list',
				description: 'List all data entries',
				getSnippets: ({ apiBaseUrl, org, repo }) => ({
					curl: `curl -X GET "${apiBaseUrl}/v1beta/repos/${org}/${repo}/data-list" \\
  -H "Authorization: Bearer pk_YOUR_API_KEY"`,
					python: `import requests

response = requests.get(
    "${apiBaseUrl}/v1beta/repos/${org}/${repo}/data-list",
    headers={"Authorization": "Bearer pk_YOUR_API_KEY"}
)
print(response.json())`,
					javascript: `const response = await fetch(
  "${apiBaseUrl}/v1beta/repos/${org}/${repo}/data-list",
  {
    headers: { "Authorization": "Bearer pk_YOUR_API_KEY" },
  }
);
const data = await response.json();
console.log(data);`,
				}),
			},
			{
				method: 'GET',
				path: '/v1beta/repos/{org}/{repo}/data/{id}',
				description: 'Get a single data entry by ID',
				getSnippets: ({ apiBaseUrl, org, repo }) => ({
					curl: `curl -X GET "${apiBaseUrl}/v1beta/repos/${org}/${repo}/data/DATA_ID" \\
  -H "Authorization: Bearer pk_YOUR_API_KEY"`,
					python: `import requests

response = requests.get(
    "${apiBaseUrl}/v1beta/repos/${org}/${repo}/data/DATA_ID",
    headers={"Authorization": "Bearer pk_YOUR_API_KEY"}
)
print(response.json())`,
					javascript: `const response = await fetch(
  "${apiBaseUrl}/v1beta/repos/${org}/${repo}/data/DATA_ID",
  {
    headers: { "Authorization": "Bearer pk_YOUR_API_KEY" },
  }
);
const data = await response.json();
console.log(data);`,
				}),
			},
			{
				method: 'POST',
				path: '/v1beta/repos/{org}/{repo}/data',
				description: 'Create a new data entry',
				getSnippets: ({ apiBaseUrl, org, repo }) => ({
					curl: `curl -X POST "${apiBaseUrl}/v1beta/repos/${org}/${repo}/data" \\
  -H "Authorization: Bearer pk_YOUR_API_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{"name": "My Data", "properties": {}}'`,
					python: `import requests

response = requests.post(
    "${apiBaseUrl}/v1beta/repos/${org}/${repo}/data",
    headers={
        "Authorization": "Bearer pk_YOUR_API_KEY",
        "Content-Type": "application/json",
    },
    json={"name": "My Data", "properties": {}}
)
print(response.json())`,
					javascript: `const response = await fetch(
  "${apiBaseUrl}/v1beta/repos/${org}/${repo}/data",
  {
    method: "POST",
    headers: {
      "Authorization": "Bearer pk_YOUR_API_KEY",
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ name: "My Data", properties: {} }),
  }
);
const data = await response.json();
console.log(data);`,
				}),
			},
			{
				method: 'PUT',
				path: '/v1beta/repos/{org}/{repo}/data/{id}',
				description: 'Update a data entry',
				getSnippets: ({ apiBaseUrl, org, repo }) => ({
					curl: `curl -X PUT "${apiBaseUrl}/v1beta/repos/${org}/${repo}/data/DATA_ID" \\
  -H "Authorization: Bearer pk_YOUR_API_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{"name": "Updated Data", "properties": {}}'`,
					python: `import requests

response = requests.put(
    "${apiBaseUrl}/v1beta/repos/${org}/${repo}/data/DATA_ID",
    headers={
        "Authorization": "Bearer pk_YOUR_API_KEY",
        "Content-Type": "application/json",
    },
    json={"name": "Updated Data", "properties": {}}
)
print(response.json())`,
					javascript: `const response = await fetch(
  "${apiBaseUrl}/v1beta/repos/${org}/${repo}/data/DATA_ID",
  {
    method: "PUT",
    headers: {
      "Authorization": "Bearer pk_YOUR_API_KEY",
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ name: "Updated Data", properties: {} }),
  }
);
const data = await response.json();
console.log(data);`,
				}),
			},
			{
				method: 'DELETE',
				path: '/v1beta/repos/{org}/{repo}/data/{id}',
				description: 'Delete a data entry',
				getSnippets: ({ apiBaseUrl, org, repo }) => ({
					curl: `curl -X DELETE "${apiBaseUrl}/v1beta/repos/${org}/${repo}/data/DATA_ID" \\
  -H "Authorization: Bearer pk_YOUR_API_KEY"`,
					python: `import requests

response = requests.delete(
    "${apiBaseUrl}/v1beta/repos/${org}/${repo}/data/DATA_ID",
    headers={"Authorization": "Bearer pk_YOUR_API_KEY"}
)
print(response.status_code)`,
					javascript: `const response = await fetch(
  "${apiBaseUrl}/v1beta/repos/${org}/${repo}/data/DATA_ID",
  {
    method: "DELETE",
    headers: { "Authorization": "Bearer pk_YOUR_API_KEY" },
  }
);
console.log(response.status);`,
				}),
			},
			{
				method: 'GET',
				path: '/v1beta/repos/{org}/{repo}/data?name=...',
				description: 'Search data by name',
				getSnippets: ({ apiBaseUrl, org, repo }) => ({
					curl: `curl -X GET "${apiBaseUrl}/v1beta/repos/${org}/${repo}/data?name=search_term" \\
  -H "Authorization: Bearer pk_YOUR_API_KEY"`,
					python: `import requests

response = requests.get(
    "${apiBaseUrl}/v1beta/repos/${org}/${repo}/data",
    params={"name": "search_term"},
    headers={"Authorization": "Bearer pk_YOUR_API_KEY"}
)
print(response.json())`,
					javascript: `const params = new URLSearchParams({ name: "search_term" });
const response = await fetch(
  \`${apiBaseUrl}/v1beta/repos/${org}/${repo}/data?\${params}\`,
  {
    headers: { "Authorization": "Bearer pk_YOUR_API_KEY" },
  }
);
const data = await response.json();
console.log(data);`,
				}),
			},
		],
	},
	{
		key: 'repository',
		endpoints: [
			{
				method: 'GET',
				path: '/v1beta/repos/{org}/{repo}',
				description: 'Get repository details',
				getSnippets: ({ apiBaseUrl, org, repo }) => ({
					curl: `curl -X GET "${apiBaseUrl}/v1beta/repos/${org}/${repo}" \\
  -H "Authorization: Bearer pk_YOUR_API_KEY"`,
					python: `import requests

response = requests.get(
    "${apiBaseUrl}/v1beta/repos/${org}/${repo}",
    headers={"Authorization": "Bearer pk_YOUR_API_KEY"}
)
print(response.json())`,
					javascript: `const response = await fetch(
  "${apiBaseUrl}/v1beta/repos/${org}/${repo}",
  {
    headers: { "Authorization": "Bearer pk_YOUR_API_KEY" },
  }
);
const data = await response.json();
console.log(data);`,
				}),
			},
		],
	},
	{
		key: 'properties',
		endpoints: [
			{
				method: 'GET',
				path: '/v1beta/repos/{org}/{repo}/properties',
				description: 'List all properties',
				getSnippets: ({ apiBaseUrl, org, repo }) => ({
					curl: `curl -X GET "${apiBaseUrl}/v1beta/repos/${org}/${repo}/properties" \\
  -H "Authorization: Bearer pk_YOUR_API_KEY"`,
					python: `import requests

response = requests.get(
    "${apiBaseUrl}/v1beta/repos/${org}/${repo}/properties",
    headers={"Authorization": "Bearer pk_YOUR_API_KEY"}
)
print(response.json())`,
					javascript: `const response = await fetch(
  "${apiBaseUrl}/v1beta/repos/${org}/${repo}/properties",
  {
    headers: { "Authorization": "Bearer pk_YOUR_API_KEY" },
  }
);
const data = await response.json();
console.log(data);`,
				}),
			},
		],
	},
	{
		key: 'export',
		endpoints: [
			{
				method: 'GET',
				path: '/v1beta/repos/{org}/{repo}/data/parquet',
				description: 'Export all data as Parquet',
				getSnippets: ({ apiBaseUrl, org, repo }) => ({
					curl: `curl -X GET "${apiBaseUrl}/v1beta/repos/${org}/${repo}/data/parquet" \\
  -H "Authorization: Bearer pk_YOUR_API_KEY" \\
  -o data.parquet`,
					python: `import requests

response = requests.get(
    "${apiBaseUrl}/v1beta/repos/${org}/${repo}/data/parquet",
    headers={"Authorization": "Bearer pk_YOUR_API_KEY"}
)
with open("data.parquet", "wb") as f:
    f.write(response.content)`,
					javascript: `const response = await fetch(
  "${apiBaseUrl}/v1beta/repos/${org}/${repo}/data/parquet",
  {
    headers: { "Authorization": "Bearer pk_YOUR_API_KEY" },
  }
);
const blob = await response.blob();
// Save blob to file`,
				}),
			},
			{
				method: 'GET',
				path: '/v1beta/repos/{org}/{repo}/data/{id}/md',
				description: 'Export a data entry as Markdown',
				getSnippets: ({ apiBaseUrl, org, repo }) => ({
					curl: `curl -X GET "${apiBaseUrl}/v1beta/repos/${org}/${repo}/data/DATA_ID/md" \\
  -H "Authorization: Bearer pk_YOUR_API_KEY"`,
					python: `import requests

response = requests.get(
    "${apiBaseUrl}/v1beta/repos/${org}/${repo}/data/DATA_ID/md",
    headers={"Authorization": "Bearer pk_YOUR_API_KEY"}
)
print(response.text)`,
					javascript: `const response = await fetch(
  "${apiBaseUrl}/v1beta/repos/${org}/${repo}/data/DATA_ID/md",
  {
    headers: { "Authorization": "Bearer pk_YOUR_API_KEY" },
  }
);
const markdown = await response.text();
console.log(markdown);`,
				}),
			},
		],
	},
]

export const HTTP_METHOD_COLORS: Record<HttpMethod, string> = {
	GET: 'bg-emerald-100 text-emerald-800 dark:bg-emerald-900/30 dark:text-emerald-400',
	POST: 'bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-400',
	PUT: 'bg-amber-100 text-amber-800 dark:bg-amber-900/30 dark:text-amber-400',
	DELETE: 'bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400',
}
