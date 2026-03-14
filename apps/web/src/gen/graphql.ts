// @ts-nocheck
import { z } from 'zod'
import { GraphQLClient } from 'graphql-request';
import { GraphQLClientRequestHeaders } from 'graphql-request/build/cjs/types';
import gql from 'graphql-tag';
export type Maybe<T> = T | null;
export type InputMaybe<T> = Maybe<T>;
export type Exact<T extends { [key: string]: unknown }> = { [K in keyof T]: T[K] };
export type MakeOptional<T, K extends keyof T> = Omit<T, K> & { [SubKey in K]?: Maybe<T[SubKey]> };
export type MakeMaybe<T, K extends keyof T> = Omit<T, K> & { [SubKey in K]: Maybe<T[SubKey]> };
export type MakeEmpty<T extends { [key: string]: unknown }, K extends keyof T> = { [_ in K]?: never };
export type Incremental<T> = T | { [P in keyof T]?: P extends ' $fragmentName' | '__typename' ? T[P] : never };
/** All built-in and custom scalars, mapped to their actual values */
export type Scalars = {
  ID: { input: string; output: string; }
  String: { input: string; output: string; }
  Boolean: { input: boolean; output: boolean; }
  Int: { input: number; output: number; }
  Float: { input: number; output: number; }
  /**
   * Implement the DateTime<Utc> scalar
   *
   * The input/output is a string in RFC3339 format.
   */
  DateTime: { input: any; output: any; }
};

export type AddDataInputData = {
  actor: Scalars['String']['input'];
  dataName: Scalars['String']['input'];
  orgUsername: Scalars['String']['input'];
  propertyData: Array<PropertyDataInputData>;
  repoUsername: Scalars['String']['input'];
};

export type ApiKeyResponse = {
  __typename?: 'ApiKeyResponse';
  apiKey: PublicApiKey;
  serviceAccount: ServiceAccount;
};

/** Input for bulk syncing ext_github property */
export type BulkSyncExtGithubInput = {
  /** ext_github property ID */
  extGithubPropertyId: Scalars['String']['input'];
  /** Organization username */
  orgUsername: Scalars['String']['input'];
  /** Repository configurations */
  repoConfigs: Array<ExtGithubRepoConfigInput>;
  /** Repository username */
  repoUsername: Scalars['String']['input'];
};

/** Result of bulk sync operation */
export type BulkSyncExtGithubResult = {
  __typename?: 'BulkSyncExtGithubResult';
  /** Number of data items skipped (already configured) */
  skippedCount: Scalars['Int']['output'];
  /** Total number of data items */
  totalCount: Scalars['Int']['output'];
  /** Number of data items updated */
  updatedCount: Scalars['Int']['output'];
};

/** Input for changing a user's role in an organization */
export type ChangeOrgMemberRoleInput = {
  /** New role to assign */
  newRole: OrgRole;
  /** Tenant/Organization ID */
  tenantId: Scalars['String']['input'];
  /** User ID whose role to change */
  userId: Scalars['String']['input'];
};

/** Input for changing a user's role in a repository */
export type ChangeRepoMemberRoleInput = {
  /** New role: "owner", "writer", or "reader" */
  newRole: Scalars['String']['input'];
  /** Repository ID */
  repoId: Scalars['String']['input'];
  /** User ID whose role to change */
  userId: Scalars['String']['input'];
};

export type ChangeRepoUsernameInput = {
  newRepoUsername: Scalars['String']['input'];
  oldRepoUsername: Scalars['String']['input'];
  orgUsername: Scalars['String']['input'];
};

/** Input for connecting to an integration. */
export type ConnectIntegrationInput = {
  /** API key or token (for non-OAuth integrations) */
  apiKey?: InputMaybe<Scalars['String']['input']>;
  /** OAuth authorization code (for OAuth integrations) */
  authCode?: InputMaybe<Scalars['String']['input']>;
  /** Integration ID to connect to */
  integrationId: Scalars['String']['input'];
};

export type CreateApiKeyInput = {
  /** TODO: add English documentation */
  name: Scalars['String']['input'];
  /** TODO: add English documentation */
  organizationUsername: Scalars['String']['input'];
  /** TODO: add English documentation */
  serviceAccountName?: InputMaybe<Scalars['String']['input']>;
};

export type CreateOperatorInput = {
  /** TODO: add English documentation */
  newOperatorOwnerId: Scalars['String']['input'];
  /** TODO: add English documentation */
  newOperatorOwnerMethod: NewOperatorOwnerMethod;
  /** TODO: add English documentation */
  newOperatorOwnerPassword?: InputMaybe<Scalars['String']['input']>;
  /** TODO: add English documentation */
  operatorAlias?: InputMaybe<Scalars['String']['input']>;
  /** TODO: add English documentation */
  operatorName: Scalars['String']['input'];
  /** TODO: add English documentation */
  platformId: Scalars['String']['input'];
};

export type CreateOrganizationInput = {
  description?: InputMaybe<Scalars['String']['input']>;
  name: Scalars['String']['input'];
  username: Scalars['String']['input'];
  website?: InputMaybe<Scalars['String']['input']>;
};

export type CreateRepoInput = {
  /**
   * TODO: add English documentation
   * TODO: add English documentation
   */
  databaseId?: InputMaybe<Scalars['String']['input']>;
  description?: InputMaybe<Scalars['String']['input']>;
  isPublic: Scalars['Boolean']['input'];
  orgUsername: Scalars['String']['input'];
  repoName: Scalars['String']['input'];
  repoUsername: Scalars['String']['input'];
  userId: Scalars['String']['input'];
};

export type CreateSourceInput = {
  name: Scalars['String']['input'];
  orgUsername: Scalars['String']['input'];
  repoUsername: Scalars['String']['input'];
  url?: InputMaybe<Scalars['String']['input']>;
};

/** Input for creating a webhook endpoint. */
export type CreateWebhookEndpointInput = {
  /** Provider-specific configuration as JSON string */
  config: Scalars['String']['input'];
  /** Events to listen for (empty = all events) */
  events: Array<Scalars['String']['input']>;
  /** Property mapping as JSON string (optional) */
  mapping?: InputMaybe<Scalars['String']['input']>;
  name: Scalars['String']['input'];
  provider: GqlProvider;
  /** Target repository ID (optional) */
  repositoryId?: InputMaybe<Scalars['String']['input']>;
};

/** Output for creating a webhook endpoint. */
export type CreateWebhookEndpointOutput = {
  __typename?: 'CreateWebhookEndpointOutput';
  endpoint: GqlWebhookEndpoint;
  /** The secret (only returned on creation) */
  secret: Scalars['String']['output'];
  webhookUrl: Scalars['String']['output'];
};

export type Data = {
  __typename?: 'Data';
  createdAt: Scalars['DateTime']['output'];
  databaseId: Scalars['String']['output'];
  id: Scalars['String']['output'];
  name: Scalars['String']['output'];
  propertyData: Array<PropertyData>;
  tenantId: Scalars['String']['output'];
  updatedAt: Scalars['DateTime']['output'];
};

export type DataList = {
  __typename?: 'DataList';
  items: Array<Data>;
  paginator: Paginator;
};

export type DateValue = {
  __typename?: 'DateValue';
  /** Date in ISO 8601 format (YYYY-MM-DD) */
  date: Scalars['String']['output'];
};

export enum DefaultRole {
  General = 'GENERAL',
  Manager = 'MANAGER',
  Owner = 'OWNER'
}

/** Input for disabling GitHub sync */
export type DisableGitHubSyncInput = {
  /** Organization username */
  orgUsername: Scalars['String']['input'];
  /** Repository username */
  repoUsername: Scalars['String']['input'];
};

/** Result of disabling GitHub sync */
export type DisableGitHubSyncResult = {
  __typename?: 'DisableGitHubSyncResult';
  /** Whether the property was actually deleted */
  deleted: Scalars['Boolean']['output'];
  /** Whether the operation succeeded */
  success: Scalars['Boolean']['output'];
};

/** Input for enabling GitHub sync */
export type EnableGitHubSyncInput = {
  /** Organization username */
  orgUsername: Scalars['String']['input'];
  /** Repository username */
  repoUsername: Scalars['String']['input'];
};

/** Result of enabling GitHub sync */
export type EnableGitHubSyncResult = {
  __typename?: 'EnableGitHubSyncResult';
  /** The created ext_github property ID */
  propertyId: Scalars['String']['output'];
  /** Whether the operation succeeded */
  success: Scalars['Boolean']['output'];
};

/** Input for enabling Linear sync */
export type EnableLinearSyncInput = {
  /** Organization username */
  orgUsername: Scalars['String']['input'];
  /** Repository username */
  repoUsername: Scalars['String']['input'];
};

/** Result of enabling Linear sync */
export type EnableLinearSyncResult = {
  __typename?: 'EnableLinearSyncResult';
  /** The created ext_linear property ID */
  propertyId: Scalars['String']['output'];
  /** Whether the operation succeeded */
  success: Scalars['Boolean']['output'];
};

/** Input for exchanging OAuth authorization code. */
export type ExchangeOAuthCodeInput = {
  /** Authorization code from OAuth callback */
  code: Scalars['String']['input'];
  /** Integration ID */
  integrationId: Scalars['String']['input'];
  /** Redirect URI used in authorization */
  redirectUri: Scalars['String']['input'];
  /** State parameter for CSRF verification */
  state?: InputMaybe<Scalars['String']['input']>;
};

/** Repository configuration for ext_github */
export type ExtGithubRepoConfigInput = {
  /** Default path (optional, supports {{name}} placeholder) */
  defaultPath?: InputMaybe<Scalars['String']['input']>;
  /** Display label (optional) */
  label?: InputMaybe<Scalars['String']['input']>;
  /** GitHub repository (owner/repo) */
  repo: Scalars['String']['input'];
};

/** Result of analyzing frontmatter across multiple files */
export type FrontmatterAnalysis = {
  __typename?: 'FrontmatterAnalysis';
  /** List of suggested properties */
  properties: Array<SuggestedProperty>;
  /** Total files analyzed */
  totalFiles: Scalars['Int']['output'];
  /** Files with valid frontmatter */
  validFiles: Scalars['Int']['output'];
};

/** Input for getting Markdown import previews */
export type GetMarkdownPreviewsInput = {
  /** GitHub repository in "owner/repo" format */
  githubRepo: Scalars['String']['input'];
  /** List of file paths to preview */
  paths: Array<Scalars['String']['input']>;
  /** Branch/tag/commit (optional) */
  refName?: InputMaybe<Scalars['String']['input']>;
};

/** GitHub OAuth authorization URL response */
export type GitHubAuthUrl = {
  __typename?: 'GitHubAuthUrl';
  /** State parameter for CSRF protection */
  state: Scalars['String']['output'];
  /** The URL to redirect the user to for authorization */
  url: Scalars['String']['output'];
};

/** GitHub OAuth connection status */
export type GitHubConnection = {
  __typename?: 'GitHubConnection';
  /** Whether GitHub is connected */
  connected: Scalars['Boolean']['output'];
  /** When the token was last refreshed */
  connectedAt?: Maybe<Scalars['DateTime']['output']>;
  /** When the token expires (if applicable) */
  expiresAt?: Maybe<Scalars['DateTime']['output']>;
  /** GitHub username (if connected) */
  username?: Maybe<Scalars['String']['output']>;
};

/** Result of listing directory contents */
export type GitHubDirectoryContents = {
  __typename?: 'GitHubDirectoryContents';
  /** List of files and directories */
  files: Array<GitHubFileInfo>;
  /** Whether the listing was truncated (more than 1000 items) */
  truncated: Scalars['Boolean']['output'];
};

/** GitHub file/directory information */
export type GitHubFileInfo = {
  __typename?: 'GitHubFileInfo';
  /** Type: "file" or "dir" */
  fileType: Scalars['String']['output'];
  /** HTML URL to view on GitHub */
  htmlUrl?: Maybe<Scalars['String']['output']>;
  /** File/directory name */
  name: Scalars['String']['output'];
  /** Full path in the repository */
  path: Scalars['String']['output'];
  /** SHA hash of the content */
  sha: Scalars['String']['output'];
  /** Size in bytes (0 for directories) */
  size: Scalars['Int']['output'];
};

/** GitHub repository information */
export type GitHubRepository = {
  __typename?: 'GitHubRepository';
  /** Default branch name */
  defaultBranch?: Maybe<Scalars['String']['output']>;
  /** Repository description */
  description?: Maybe<Scalars['String']['output']>;
  /** Full name (owner/repo format) */
  fullName: Scalars['String']['output'];
  /** HTML URL to the repository */
  htmlUrl: Scalars['String']['output'];
  /** Repository ID */
  id: Scalars['String']['output'];
  /** Repository name */
  name: Scalars['String']['output'];
  /** Whether the repository is private */
  private: Scalars['Boolean']['output'];
};

/** Tenant's connection to an integration. */
export type GqlConnection = {
  __typename?: 'GqlConnection';
  /** When the connection was created */
  connectedAt: Scalars['DateTime']['output'];
  /** Error message if status is Error */
  errorMessage?: Maybe<Scalars['String']['output']>;
  /** External account identifier (e.g., GitHub username) */
  externalAccountId?: Maybe<Scalars['String']['output']>;
  /** External account name for display */
  externalAccountName?: Maybe<Scalars['String']['output']>;
  /** Unique identifier */
  id: Scalars['String']['output'];
  /** Integration ID this connection is for */
  integrationId: Scalars['String']['output'];
  /** When the connection was last synced */
  lastSyncedAt?: Maybe<Scalars['DateTime']['output']>;
  /** Provider name */
  provider: GqlProvider;
  /** Current status */
  status: GqlConnectionStatus;
  /** Tenant ID that owns this connection */
  tenantId: Scalars['String']['output'];
  /** When OAuth token expires (if applicable) */
  tokenExpiresAt?: Maybe<Scalars['DateTime']['output']>;
};

/** Actions for connection status changes. */
export enum GqlConnectionAction {
  /** Disconnect/remove the connection */
  Disconnect = 'DISCONNECT',
  /** Pause the connection */
  Pause = 'PAUSE',
  /** Re-authorize (refresh OAuth token) */
  Reauthorize = 'REAUTHORIZE',
  /** Resume a paused connection */
  Resume = 'RESUME'
}

/** Connection status for GraphQL. */
export enum GqlConnectionStatus {
  /** Connection is active and working */
  Active = 'ACTIVE',
  /** Connection has been disconnected */
  Disconnected = 'DISCONNECTED',
  /** Connection has an error */
  Error = 'ERROR',
  /** OAuth token has expired, needs re-authorization */
  Expired = 'EXPIRED',
  /** Connection is paused by the user */
  Paused = 'PAUSED'
}

/** Endpoint status enum for GraphQL. */
export enum GqlEndpointStatus {
  Active = 'ACTIVE',
  Disabled = 'DISABLED',
  Paused = 'PAUSED'
}

/** Integration available in the marketplace. */
export type GqlIntegration = {
  __typename?: 'GqlIntegration';
  /** Category */
  category: GqlIntegrationCategory;
  /** Description of the integration */
  description: Scalars['String']['output'];
  /** Icon URL or identifier */
  icon?: Maybe<Scalars['String']['output']>;
  /** Unique identifier */
  id: Scalars['String']['output'];
  /** Whether this integration is enabled in the marketplace */
  isEnabled: Scalars['Boolean']['output'];
  /** Whether this is a featured/recommended integration */
  isFeatured: Scalars['Boolean']['output'];
  /** Display name */
  name: Scalars['String']['output'];
  /** OAuth configuration (if applicable) */
  oauthConfig?: Maybe<GqlOAuthConfig>;
  /** The provider this integration is for */
  provider: GqlProvider;
  /** Whether OAuth is required for this integration */
  requiresOauth: Scalars['Boolean']['output'];
  /** Supported object types (e.g., "customers", "orders", "products") */
  supportedObjects: Array<Scalars['String']['output']>;
  /** Sync capabilities */
  syncCapability: GqlSyncCapability;
};

/** Integration category for GraphQL. */
export enum GqlIntegrationCategory {
  /** Code and repository management (GitHub, GitLab) */
  CodeManagement = 'CODE_MANAGEMENT',
  /** Databases and content (Notion, Airtable) */
  ContentManagement = 'CONTENT_MANAGEMENT',
  /** Customer relationship management (HubSpot, Salesforce) */
  Crm = 'CRM',
  /** Custom/generic integrations */
  Custom = 'CUSTOM',
  /** E-commerce and inventory (Square, Shopify) */
  Ecommerce = 'ECOMMERCE',
  /** Payment processing (Stripe, Square) */
  Payments = 'PAYMENTS',
  /** Issue and project tracking (Linear, Jira) */
  ProjectManagement = 'PROJECT_MANAGEMENT'
}

/** OAuth configuration for an integration. */
export type GqlOAuthConfig = {
  __typename?: 'GqlOAuthConfig';
  /** OAuth authorization URL */
  authUrl: Scalars['String']['output'];
  /** Required OAuth scopes for this integration */
  scopes: Array<Scalars['String']['output']>;
  /** Whether refresh tokens are supported */
  supportsRefresh: Scalars['Boolean']['output'];
  /** OAuth token URL */
  tokenUrl: Scalars['String']['output'];
};

/** Processing stats GraphQL type. */
export type GqlProcessingStats = {
  __typename?: 'GqlProcessingStats';
  created: Scalars['Int']['output'];
  deleted: Scalars['Int']['output'];
  skipped: Scalars['Int']['output'];
  total: Scalars['Int']['output'];
  updated: Scalars['Int']['output'];
};

/** Processing status enum for GraphQL. */
export enum GqlProcessingStatus {
  Completed = 'COMPLETED',
  Failed = 'FAILED',
  Pending = 'PENDING',
  Processing = 'PROCESSING',
  Skipped = 'SKIPPED'
}

/** Provider type enum for GraphQL. */
export enum GqlProvider {
  Airtable = 'AIRTABLE',
  Custom = 'CUSTOM',
  Generic = 'GENERIC',
  Github = 'GITHUB',
  Hubspot = 'HUBSPOT',
  Linear = 'LINEAR',
  Notion = 'NOTION',
  Square = 'SQUARE',
  Stripe = 'STRIPE'
}

/** Sync capability for GraphQL. */
export enum GqlSyncCapability {
  /** Can both receive and push data */
  Bidirectional = 'BIDIRECTIONAL',
  /** Can receive data from external service (webhooks) */
  Inbound = 'INBOUND',
  /** Can push data to external service */
  Outbound = 'OUTBOUND'
}

/** Sync operation for GraphQL. */
export type GqlSyncOperation = {
  __typename?: 'GqlSyncOperation';
  completedAt?: Maybe<Scalars['DateTime']['output']>;
  endpointId: Scalars['String']['output'];
  errorMessage?: Maybe<Scalars['String']['output']>;
  id: Scalars['String']['output'];
  operationType: GqlSyncOperationType;
  progress?: Maybe<Scalars['String']['output']>;
  startedAt: Scalars['DateTime']['output'];
  stats?: Maybe<GqlProcessingStats>;
  status: GqlSyncOperationStatus;
};

/** Sync operation status enum for GraphQL. */
export enum GqlSyncOperationStatus {
  Cancelled = 'CANCELLED',
  Completed = 'COMPLETED',
  Failed = 'FAILED',
  Queued = 'QUEUED',
  Running = 'RUNNING'
}

/** Sync operation type enum for GraphQL. */
export enum GqlSyncOperationType {
  InitialSync = 'INITIAL_SYNC',
  OnDemandPull = 'ON_DEMAND_PULL',
  ScheduledSync = 'SCHEDULED_SYNC',
  Webhook = 'WEBHOOK'
}

/** Webhook endpoint GraphQL type. */
export type GqlWebhookEndpoint = {
  __typename?: 'GqlWebhookEndpoint';
  config: Scalars['String']['output'];
  createdAt: Scalars['DateTime']['output'];
  events: Array<Scalars['String']['output']>;
  id: Scalars['String']['output'];
  mapping?: Maybe<Scalars['String']['output']>;
  name: Scalars['String']['output'];
  provider: GqlProvider;
  repositoryId?: Maybe<Scalars['String']['output']>;
  status: GqlEndpointStatus;
  tenantId: Scalars['String']['output'];
  updatedAt: Scalars['DateTime']['output'];
  webhookUrl: Scalars['String']['output'];
};

/** Webhook event GraphQL type. */
export type GqlWebhookEvent = {
  __typename?: 'GqlWebhookEvent';
  endpointId: Scalars['String']['output'];
  errorMessage?: Maybe<Scalars['String']['output']>;
  eventType: Scalars['String']['output'];
  id: Scalars['String']['output'];
  payload: Scalars['String']['output'];
  processedAt?: Maybe<Scalars['DateTime']['output']>;
  provider: GqlProvider;
  receivedAt: Scalars['DateTime']['output'];
  retryCount: Scalars['Int']['output'];
  signatureValid: Scalars['Boolean']['output'];
  stats?: Maybe<GqlProcessingStats>;
  status: GqlProcessingStatus;
};

export type HtmlValue = {
  __typename?: 'HtmlValue';
  html: Scalars['String']['output'];
};

export type IdOrEmail =
  { email: Scalars['String']['input']; id?: never; }
  |  { email?: never; id: Scalars['String']['input']; };

export type IdType = {
  __typename?: 'IdType';
  autoGenerate: Scalars['Boolean']['output'];
};

export type IdValue = {
  __typename?: 'IdValue';
  id: Scalars['String']['output'];
};

export type ImageValue = {
  __typename?: 'ImageValue';
  /** Image URL */
  url: Scalars['String']['output'];
};

/** Error during import */
export type ImportError = {
  __typename?: 'ImportError';
  /** Error message */
  message: Scalars['String']['output'];
  /** File path that caused the error */
  path: Scalars['String']['output'];
};

/** Input for importing Markdown files from GitHub */
export type ImportMarkdownFromGitHubInput = {
  /** Property name for markdown content */
  contentPropertyName: Scalars['String']['input'];
  /**
   * Whether to enable GitHub sync (default: true)
   * If false, ext_github property will be created but without repo config
   */
  enableGithubSync?: InputMaybe<Scalars['Boolean']['input']>;
  /** GitHub repository in "owner/repo" format */
  githubRepo: Scalars['String']['input'];
  /** Organization username */
  orgUsername: Scalars['String']['input'];
  /** List of file paths to import */
  paths: Array<Scalars['String']['input']>;
  /** Property mappings from frontmatter */
  propertyMappings: Array<PropertyMappingInput>;
  /** Branch/tag/commit (optional) */
  refName?: InputMaybe<Scalars['String']['input']>;
  /** Repository name (for creating new repo) */
  repoName?: InputMaybe<Scalars['String']['input']>;
  /** Repository username (will be created if it doesn't exist) */
  repoUsername: Scalars['String']['input'];
  /** Whether to skip files that already exist (by ext_github path) */
  skipExisting?: InputMaybe<Scalars['Boolean']['input']>;
};

/** Result of importing Markdown files */
export type ImportMarkdownResult = {
  __typename?: 'ImportMarkdownResult';
  /** IDs of created/updated data items */
  dataIds: Array<Scalars['String']['output']>;
  /** List of errors encountered */
  errors: Array<ImportError>;
  /** Number of files successfully imported */
  importedCount: Scalars['Int']['output'];
  /** ID of the repository (created or existing) */
  repoId: Scalars['String']['output'];
  /** Number of files skipped */
  skippedCount: Scalars['Int']['output'];
  /** Number of files updated (already existed) */
  updatedCount: Scalars['Int']['output'];
};

/** Input for initializing OAuth authorization. */
export type InitOAuthInput = {
  /** Integration ID to connect */
  integrationId: Scalars['String']['input'];
  /** Optional redirect URI (defaults to backend callback URL if not provided) */
  redirectUri?: InputMaybe<Scalars['String']['input']>;
  /**
   * Optional custom state for CSRF protection (will be generated if not provided).
   * Frontend can encode additional data (e.g., tenant_id, integration_id) in this field.
   */
  state?: InputMaybe<Scalars['String']['input']>;
};

export type IntegerValue = {
  __typename?: 'IntegerValue';
  number: Scalars['String']['output'];
};

/** Input for inviting a user to a repository */
export type InviteRepoMemberInput = {
  /** Organization username */
  orgUsername: Scalars['String']['input'];
  /** Repository ID */
  repoId: Scalars['String']['input'];
  /** Repository username */
  repoUsername: Scalars['String']['input'];
  /** Role to assign: "owner", "writer", or "reader" */
  role: Scalars['String']['input'];
  /** Username or email of the user to invite */
  usernameOrEmail: Scalars['String']['input'];
};

export type JsonType = {
  __typename?: 'JsonType';
  /** TODO: add English documentation */
  json: Scalars['String']['output'];
};

/** Linear Issue for GraphQL */
export type LinearIssue = {
  __typename?: 'LinearIssue';
  /** Assignee name */
  assigneeName?: Maybe<Scalars['String']['output']>;
  /** Issue ID */
  id: Scalars['String']['output'];
  /** Issue identifier (e.g., "ENG-123") */
  identifier: Scalars['String']['output'];
  /** Issue state name */
  stateName?: Maybe<Scalars['String']['output']>;
  /** Issue title */
  title: Scalars['String']['output'];
  /** Issue URL */
  url?: Maybe<Scalars['String']['output']>;
};

/** Linear Project for GraphQL */
export type LinearProject = {
  __typename?: 'LinearProject';
  /** Project ID */
  id: Scalars['String']['output'];
  /** Project name */
  name: Scalars['String']['output'];
};

/** Linear Team for GraphQL */
export type LinearTeam = {
  __typename?: 'LinearTeam';
  /** Team ID */
  id: Scalars['String']['output'];
  /** Team key (e.g., "ENG") */
  key: Scalars['String']['output'];
  /** Team name */
  name: Scalars['String']['output'];
};

/** Input for listing GitHub directory contents */
export type ListGitHubDirectoryInput = {
  /** GitHub repository in "owner/repo" format */
  githubRepo: Scalars['String']['input'];
  /** Path to the directory (empty for root) */
  path: Scalars['String']['input'];
  /** Whether to include subdirectories recursively */
  recursive?: InputMaybe<Scalars['Boolean']['input']>;
  /** Branch/tag/commit (optional, defaults to default branch) */
  refName?: InputMaybe<Scalars['String']['input']>;
};

/**
 * # Location
 * TODO: add English documentation
 */
export type Location = {
  /** TODO: add English documentation */
  latitude: Scalars['Float']['input'];
  /** TODO: add English documentation */
  longitude: Scalars['Float']['input'];
};

export type LocationValue = {
  __typename?: 'LocationValue';
  /**
   * TODO: add English documentation
   * -90.0 ~ 90.0
   */
  latitude: Scalars['Float']['output'];
  /**
   * TODO: add English documentation
   * -180.0 ~ 180.0
   */
  longitude: Scalars['Float']['output'];
};

/** Preview of a Markdown file for import */
export type MarkdownImportPreview = {
  __typename?: 'MarkdownImportPreview';
  /** Preview of the markdown body (first 500 chars) */
  bodyPreview: Scalars['String']['output'];
  /** Parsed frontmatter as JSON string */
  frontmatterJson?: Maybe<Scalars['String']['output']>;
  /** List of frontmatter keys found */
  frontmatterKeys: Array<Scalars['String']['output']>;
  /** Error message if parsing failed */
  parseError?: Maybe<Scalars['String']['output']>;
  /** File path in the repository */
  path: Scalars['String']['output'];
  /** SHA hash for change detection */
  sha: Scalars['String']['output'];
  /** Suggested data name (from title, h1, or filename) */
  suggestedName: Scalars['String']['output'];
};

export type MarkdownValue = {
  __typename?: 'MarkdownValue';
  markdown: Scalars['String']['output'];
};

export type MultiSelectType = {
  __typename?: 'MultiSelectType';
  options: Array<SelectItem>;
};

export type MultiSelectValue = {
  __typename?: 'MultiSelectValue';
  optionIds: Array<Scalars['String']['output']>;
};

export type Mutation = {
  __typename?: 'Mutation';
  addData: Data;
  /** TODO: add English documentation */
  addProperty: Property;
  /** [LIBRARY-API] Bulk sync ext_github property for all data items */
  bulkSyncExtGithub: BulkSyncExtGithubResult;
  /**
   * [LIBRARY-API] Change a user's role in an organization
   *
   * Updates the user's DefaultRole and manages library-specific policies:
   * - If upgrading to Owner, attaches repo owner policy for full repo access
   * - If downgrading from Owner, detaches repo owner policy
   */
  changeOrgMemberRole: User;
  /** [LIBRARY-API] Change a user's role in a repository */
  changeRepoMemberRole: Scalars['Boolean']['output'];
  /** TODO: add English documentation */
  changeRepoUsername: Repo;
  /** TODO: add English documentation */
  check: Scalars['String']['output'];
  /**
   * Connect to an integration.
   *
   * Creates a new connection to the specified integration.
   * For OAuth integrations, provide the authorization code.
   * For API key integrations, provide the API key.
   */
  connectIntegration: GqlConnection;
  /** TODO: add English documentation */
  createApiKey: ApiKeyResponse;
  /** TODO: add English documentation */
  createData: Data;
  /** [AUTH] Create operator via SDK REST call */
  createOperator: Operator;
  /** TODO: add English documentation */
  createOrganization: Organization;
  /** TODO: add English documentation */
  createRepo: Repo;
  /** TODO: add English documentation */
  createSource: Source;
  /**
   * Create a new webhook endpoint.
   *
   * Returns the created endpoint along with the webhook URL and secret.
   * **Important**: The secret is only returned once. Store it securely.
   */
  createWebhookEndpoint: CreateWebhookEndpointOutput;
  /** Delete a connection permanently. */
  deleteConnection: Scalars['Boolean']['output'];
  /** TODO: add English documentation */
  deleteProperty: Scalars['String']['output'];
  /** TODO: add English documentation */
  deleteRepo: Scalars['String']['output'];
  /** TODO: add English documentation */
  deleteSource: Scalars['String']['output'];
  /** Delete a webhook endpoint. */
  deleteWebhookEndpoint: Scalars['Boolean']['output'];
  /** [LIBRARY-API] Disable GitHub sync by deleting the ext_github property */
  disableGithubSync: DisableGitHubSyncResult;
  /** [LIBRARY-API] Enable GitHub sync by creating the ext_github property (system use only) */
  enableGithubSync: EnableGitHubSyncResult;
  /** [LIBRARY-API] Enable Linear sync by creating the ext_linear property (system use only) */
  enableLinearSync: EnableLinearSyncResult;
  /**
   * Exchange OAuth authorization code for tokens.
   *
   * Creates or updates a connection with the tokens from the OAuth provider.
   */
  exchangeOauthCode: GqlConnection;
  /**
   * [LIBRARY-API] Get GitHub OAuth authorization URL
   *
   * Signs the state parameter with HMAC-SHA256 for CSRF protection.
   * The signed state will be validated in github_exchange_token.
   */
  githubAuthUrl: GitHubAuthUrl;
  /** [LIBRARY-API] Disconnect GitHub OAuth */
  githubDisconnect: Scalars['Boolean']['output'];
  /**
   * [LIBRARY-API] Exchange GitHub OAuth code for token
   *
   * Validates the signed state parameter for CSRF protection before
   * exchanging the code.
   */
  githubExchangeToken: GitHubConnection;
  /** [LIBRARY-API] Import Markdown files from GitHub */
  importMarkdownFromGithub: ImportMarkdownResult;
  /**
   * Initialize OAuth authorization flow.
   *
   * Returns the authorization URL to redirect the user to.
   */
  initOauth: OauthInitOutput;
  /** [LIBRARY-API] Invite a user to a repository with a specific role (owner/writer/reader) */
  inviteRepoMember: Scalars['Boolean']['output'];
  /**
   * [LIBRARY-API] Invite a user to an organization with library-specific policy setup
   *
   * This wraps auth's InviteUser and additionally:
   * - Attaches LibraryUserPolicy to the invited user
   * - If the invited user becomes org owner, attaches repo owner policy
   */
  inviteUser: User;
  /** [LIBRARY-API] Remove a user from a repository */
  removeRepoMember: Scalars['Boolean']['output'];
  /** Retry a failed webhook event. */
  retryWebhookEvent: GqlWebhookEvent;
  /** Send a test webhook to an endpoint. */
  sendTestWebhook: SendTestWebhookOutput;
  /** [AUTH] Sign in or sign up via platform access token (library) */
  signIn: User;
  /** Start initial sync operation. */
  startInitialSync: GqlSyncOperation;
  /** [LIBRARY-API] Sync data to GitHub */
  syncDataToGithub: SyncResult;
  /** Trigger on-demand pull sync. */
  triggerSync: GqlSyncOperation;
  /**
   * Update a connection's status.
   *
   * Allows pausing, resuming, disconnecting, or reauthorizing a connection.
   */
  updateConnection: GqlConnection;
  /** TODO: add English documentation */
  updateData: Data;
  /** TODO: add English documentation */
  updateOrganization: Organization;
  /** TODO: add English documentation */
  updateProperty: Property;
  /** TODO: add English documentation */
  updateRepo: Repo;
  /** TODO: add English documentation */
  updateSource: Source;
  /** Update webhook endpoint configuration. */
  updateWebhookEndpointConfig: GqlWebhookEndpoint;
  /** Update webhook endpoint events. */
  updateWebhookEndpointEvents: GqlWebhookEndpoint;
  /** Update webhook endpoint property mapping. */
  updateWebhookEndpointMapping: GqlWebhookEndpoint;
  /** Update webhook endpoint status. */
  updateWebhookEndpointStatus: GqlWebhookEndpoint;
  /** [AUTH] Verify the token and return the user */
  verify: User;
};


export type MutationAddDataArgs = {
  input: AddDataInputData;
};


export type MutationAddPropertyArgs = {
  input: PropertyInput;
};


export type MutationBulkSyncExtGithubArgs = {
  input: BulkSyncExtGithubInput;
};


export type MutationChangeOrgMemberRoleArgs = {
  input: ChangeOrgMemberRoleInput;
};


export type MutationChangeRepoMemberRoleArgs = {
  input: ChangeRepoMemberRoleInput;
};


export type MutationChangeRepoUsernameArgs = {
  input: ChangeRepoUsernameInput;
};


export type MutationConnectIntegrationArgs = {
  input: ConnectIntegrationInput;
};


export type MutationCreateApiKeyArgs = {
  input: CreateApiKeyInput;
};


export type MutationCreateDataArgs = {
  input: AddDataInputData;
};


export type MutationCreateOperatorArgs = {
  input: CreateOperatorInput;
};


export type MutationCreateOrganizationArgs = {
  input: CreateOrganizationInput;
};


export type MutationCreateRepoArgs = {
  input: CreateRepoInput;
};


export type MutationCreateSourceArgs = {
  input: CreateSourceInput;
};


export type MutationCreateWebhookEndpointArgs = {
  input: CreateWebhookEndpointInput;
};


export type MutationDeleteConnectionArgs = {
  connectionId: Scalars['String']['input'];
};


export type MutationDeletePropertyArgs = {
  orgUsername: Scalars['String']['input'];
  propertyId: Scalars['String']['input'];
  repoUsername: Scalars['String']['input'];
};


export type MutationDeleteRepoArgs = {
  orgUsername: Scalars['String']['input'];
  repoUsername: Scalars['String']['input'];
};


export type MutationDeleteSourceArgs = {
  orgUsername: Scalars['String']['input'];
  repoUsername: Scalars['String']['input'];
  sourceId: Scalars['String']['input'];
};


export type MutationDeleteWebhookEndpointArgs = {
  endpointId: Scalars['String']['input'];
};


export type MutationDisableGithubSyncArgs = {
  input: DisableGitHubSyncInput;
};


export type MutationEnableGithubSyncArgs = {
  input: EnableGitHubSyncInput;
};


export type MutationEnableLinearSyncArgs = {
  input: EnableLinearSyncInput;
};


export type MutationExchangeOauthCodeArgs = {
  input: ExchangeOAuthCodeInput;
};


export type MutationGithubAuthUrlArgs = {
  state: Scalars['String']['input'];
};


export type MutationGithubExchangeTokenArgs = {
  code: Scalars['String']['input'];
  state: Scalars['String']['input'];
};


export type MutationImportMarkdownFromGithubArgs = {
  input: ImportMarkdownFromGitHubInput;
};


export type MutationInitOauthArgs = {
  input: InitOAuthInput;
};


export type MutationInviteRepoMemberArgs = {
  input: InviteRepoMemberInput;
};


export type MutationInviteUserArgs = {
  invitee: IdOrEmail;
  notifyUser?: InputMaybe<Scalars['Boolean']['input']>;
  platformId?: InputMaybe<Scalars['String']['input']>;
  role?: InputMaybe<OrgRole>;
  tenantId: Scalars['String']['input'];
};


export type MutationRemoveRepoMemberArgs = {
  input: RemoveRepoMemberInput;
};


export type MutationRetryWebhookEventArgs = {
  eventId: Scalars['String']['input'];
};


export type MutationSendTestWebhookArgs = {
  endpointId: Scalars['String']['input'];
  eventType: Scalars['String']['input'];
};


export type MutationSignInArgs = {
  accessToken: Scalars['String']['input'];
  allowSignUp?: InputMaybe<Scalars['Boolean']['input']>;
  platformId: Scalars['String']['input'];
};


export type MutationStartInitialSyncArgs = {
  input: StartInitialSyncInput;
};


export type MutationSyncDataToGithubArgs = {
  input: SyncToGitHubInput;
};


export type MutationTriggerSyncArgs = {
  input: TriggerSyncInput;
};


export type MutationUpdateConnectionArgs = {
  action: GqlConnectionAction;
  connectionId: Scalars['String']['input'];
};


export type MutationUpdateDataArgs = {
  input: UpdateDataInputData;
};


export type MutationUpdateOrganizationArgs = {
  input: UpdateOrganizationInput;
};


export type MutationUpdatePropertyArgs = {
  id: Scalars['String']['input'];
  input: PropertyInput;
};


export type MutationUpdateRepoArgs = {
  input: UpdateRepoInput;
};


export type MutationUpdateSourceArgs = {
  input: UpdateSourceInput;
};


export type MutationUpdateWebhookEndpointConfigArgs = {
  input: UpdateEndpointConfigInput;
};


export type MutationUpdateWebhookEndpointEventsArgs = {
  input: UpdateEndpointEventsInput;
};


export type MutationUpdateWebhookEndpointMappingArgs = {
  input: UpdateEndpointMappingInput;
};


export type MutationUpdateWebhookEndpointStatusArgs = {
  input: UpdateEndpointStatusInput;
};


export type MutationVerifyArgs = {
  token: Scalars['String']['input'];
};

/** Owner information passed when creating a new Operator */
export enum NewOperatorOwnerMethod {
  Create = 'CREATE',
  Inherit = 'INHERIT'
}

/** Output for OAuth authorization initialization. */
export type OauthInitOutput = {
  __typename?: 'OauthInitOutput';
  /** URL to redirect user to for authorization */
  authorizationUrl: Scalars['String']['output'];
  /** State parameter for CSRF protection */
  state: Scalars['String']['output'];
};

export type Operator = {
  __typename?: 'Operator';
  createdAt: Scalars['DateTime']['output'];
  id: Scalars['String']['output'];
  name: Scalars['String']['output'];
  operatorName: Scalars['String']['output'];
  platformTenantId: Scalars['String']['output'];
  updatedAt: Scalars['DateTime']['output'];
};

export type OptionInput = {
  identifier: Scalars['String']['input'];
  label: Scalars['String']['input'];
};

/** Role for library organization members. */
export enum OrgRole {
  /** General member with basic permissions. */
  General = 'GENERAL',
  /** Manager with elevated permissions. */
  Manager = 'MANAGER',
  /** Organization owner with full access to all repositories. */
  Owner = 'OWNER'
}

export type Organization = {
  __typename?: 'Organization';
  description?: Maybe<Scalars['String']['output']>;
  id: Scalars['String']['output'];
  name: Scalars['String']['output'];
  repos: Array<Repo>;
  username: Scalars['String']['output'];
  users: Array<User>;
  website?: Maybe<Scalars['String']['output']>;
};

export type Paginator = {
  __typename?: 'Paginator';
  currentPage: Scalars['Int']['output'];
  itemsPerPage: Scalars['Int']['output'];
  totalItems: Scalars['Int']['output'];
  totalPages: Scalars['Int']['output'];
};

/**
 * Source of a repository permission
 *
 * Indicates whether a permission was explicitly assigned to the repository
 * or inherited from the organization-level role.
 */
export enum PermissionSource {
  /** Permission inherited from organization-level role (org owner) */
  Org = 'ORG',
  /** Permission explicitly assigned to this repository */
  Repo = 'REPO'
}

export type Property = {
  __typename?: 'Property';
  databaseId: Scalars['String']['output'];
  id: Scalars['String']['output'];
  isIndexed: Scalars['Boolean']['output'];
  /** TODO: add English documentation */
  meta?: Maybe<PropertyTypeMeta>;
  name: Scalars['String']['output'];
  propertyNum: Scalars['Int']['output'];
  tenantId: Scalars['String']['output'];
  /**
   * TODO: add English documentation
   * STRING, INTEGER, HTML, MARKDOWN, RELATION, SELECT, MULTI_SELECT
   */
  typ: PropertyType;
};

export type PropertyData = {
  __typename?: 'PropertyData';
  propertyId: Scalars['String']['output'];
  value: PropertyDataValue;
};

export type PropertyDataInputData = {
  propertyId: Scalars['String']['input'];
  value: PropertyDataValueInputData;
};

export type PropertyDataValue = DateValue | HtmlValue | IdValue | ImageValue | IntegerValue | LocationValue | MarkdownValue | MultiSelectValue | RelationValue | SelectValue | StringValue;

export type PropertyDataValueInputData =
  { date: Scalars['String']['input']; html?: never; image?: never; integer?: never; location?: never; markdown?: never; multiSelect?: never; relation?: never; select?: never; string?: never; }
  |  { date?: never; html: Scalars['String']['input']; image?: never; integer?: never; location?: never; markdown?: never; multiSelect?: never; relation?: never; select?: never; string?: never; }
  |  { date?: never; html?: never; image: Scalars['String']['input']; integer?: never; location?: never; markdown?: never; multiSelect?: never; relation?: never; select?: never; string?: never; }
  |  { date?: never; html?: never; image?: never; integer: Scalars['String']['input']; location?: never; markdown?: never; multiSelect?: never; relation?: never; select?: never; string?: never; }
  |  { date?: never; html?: never; image?: never; integer?: never; location: Location; markdown?: never; multiSelect?: never; relation?: never; select?: never; string?: never; }
  |  { date?: never; html?: never; image?: never; integer?: never; location?: never; markdown: Scalars['String']['input']; multiSelect?: never; relation?: never; select?: never; string?: never; }
  |  { date?: never; html?: never; image?: never; integer?: never; location?: never; markdown?: never; multiSelect: Array<Scalars['String']['input']>; relation?: never; select?: never; string?: never; }
  |  { date?: never; html?: never; image?: never; integer?: never; location?: never; markdown?: never; multiSelect?: never; relation: Array<Scalars['String']['input']>; select?: never; string?: never; }
  |  { date?: never; html?: never; image?: never; integer?: never; location?: never; markdown?: never; multiSelect?: never; relation?: never; select: Scalars['String']['input']; string?: never; }
  |  { date?: never; html?: never; image?: never; integer?: never; location?: never; markdown?: never; multiSelect?: never; relation?: never; select?: never; string: Scalars['String']['input']; };

export type PropertyInput = {
  meta?: InputMaybe<PropertyMetaInput>;
  orgUsername: Scalars['String']['input'];
  propertyName: Scalars['String']['input'];
  propertyType: PropertyType;
  repoUsername: Scalars['String']['input'];
};

/** Property mapping for import */
export type PropertyMappingInput = {
  /** Frontmatter key name */
  frontmatterKey: Scalars['String']['input'];
  /** Property name to create/use */
  propertyName: Scalars['String']['input'];
  /** Property type */
  propertyType: PropertyType;
  /** Select options (if type is Select) */
  selectOptions?: InputMaybe<Array<Scalars['String']['input']>>;
};

export type PropertyMetaInput =
  /** TODO: add English documentation */
  { id: Scalars['Boolean']['input']; json?: never; multiSelect?: never; relation?: never; select?: never; }
  |  /** TODO: add English documentation */
  { id?: never; json: Scalars['String']['input']; multiSelect?: never; relation?: never; select?: never; }
  |  /** TODO: add English documentation */
  { id?: never; json?: never; multiSelect: Array<OptionInput>; relation?: never; select?: never; }
  |  /** TODO: add English documentation */
  { id?: never; json?: never; multiSelect?: never; relation: Scalars['String']['input']; select?: never; }
  |  /** TODO: add English documentation */
  { id?: never; json?: never; multiSelect?: never; relation?: never; select: Array<OptionInput>; };

export enum PropertyType {
  Date = 'DATE',
  /** @deprecated Use MARKDOWN instead of HTML. */
  Html = 'HTML',
  Id = 'ID',
  Image = 'IMAGE',
  Integer = 'INTEGER',
  Location = 'LOCATION',
  Markdown = 'MARKDOWN',
  MultiSelect = 'MULTI_SELECT',
  Relation = 'RELATION',
  Select = 'SELECT',
  String = 'STRING'
}

export type PropertyTypeMeta = IdType | JsonType | MultiSelectType | RelationType | SelectType;

export type PublicApiKey = {
  __typename?: 'PublicApiKey';
  createdAt: Scalars['DateTime']['output'];
  id: Scalars['String']['output'];
  name: Scalars['String']['output'];
  serviceAccountId: Scalars['String']['output'];
  tenantId: Scalars['String']['output'];
  value: Scalars['String']['output'];
};

export type Query = {
  __typename?: 'Query';
  apiKeys: Array<PublicApiKey>;
  /** Get a single connection by ID. */
  connection?: Maybe<GqlConnection>;
  /** Get all connections for a tenant. */
  connections: Array<GqlConnection>;
  data: Data;
  dataList: DataList;
  errTest: Scalars['String']['output'];
  /** [LIBRARY-API] Analyze frontmatter across multiple files */
  githubAnalyzeFrontmatter: FrontmatterAnalysis;
  /** [LIBRARY-API] Get GitHub connection status */
  githubConnection: GitHubConnection;
  /** [LIBRARY-API] Get previews of Markdown files for import */
  githubGetMarkdownPreviews: Array<MarkdownImportPreview>;
  /** [LIBRARY-API] List directory contents from a GitHub repository */
  githubListDirectoryContents: GitHubDirectoryContents;
  /** [LIBRARY-API] List GitHub repositories accessible to the user */
  githubListRepositories: Array<GitHubRepository>;
  /** Get a single integration by ID. */
  integration?: Maybe<GqlIntegration>;
  /** Get an integration by provider. */
  integrationByProvider?: Maybe<GqlIntegration>;
  /** Get all available integrations in the marketplace. */
  integrations: Array<GqlIntegration>;
  /** [LIBRARY-API] List Linear issues */
  linearListIssues: Array<LinearIssue>;
  /** [LIBRARY-API] List Linear projects */
  linearListProjects: Array<LinearProject>;
  /** [LIBRARY-API] List Linear teams */
  linearListTeams: Array<LinearTeam>;
  me: User;
  organization: Organization;
  properties: Array<Property>;
  repo: Repo;
  source: Source;
  /** Get a single sync operation by ID. */
  syncOperation?: Maybe<GqlSyncOperation>;
  /** Get sync operations for an endpoint. */
  syncOperations: Array<GqlSyncOperation>;
  /** Get a webhook endpoint by ID. */
  webhookEndpoint?: Maybe<GqlWebhookEndpoint>;
  /** List webhook endpoints for a tenant. */
  webhookEndpoints: Array<GqlWebhookEndpoint>;
  /** Get a single webhook event by ID. */
  webhookEvent?: Maybe<GqlWebhookEvent>;
  /** Get webhook events for an endpoint. */
  webhookEvents: Array<GqlWebhookEvent>;
};


export type QueryApiKeysArgs = {
  orgUsername: Scalars['String']['input'];
};


export type QueryConnectionArgs = {
  id: Scalars['String']['input'];
};


export type QueryConnectionsArgs = {
  activeOnly?: InputMaybe<Scalars['Boolean']['input']>;
  tenantId: Scalars['String']['input'];
};


export type QueryDataArgs = {
  dataId: Scalars['String']['input'];
  orgUsername: Scalars['String']['input'];
  repoUsername: Scalars['String']['input'];
};


export type QueryDataListArgs = {
  orgUsername: Scalars['String']['input'];
  page?: InputMaybe<Scalars['Int']['input']>;
  pageSize?: InputMaybe<Scalars['Int']['input']>;
  repoUsername: Scalars['String']['input'];
};


export type QueryGithubAnalyzeFrontmatterArgs = {
  input: GetMarkdownPreviewsInput;
};


export type QueryGithubGetMarkdownPreviewsArgs = {
  input: GetMarkdownPreviewsInput;
};


export type QueryGithubListDirectoryContentsArgs = {
  input: ListGitHubDirectoryInput;
};


export type QueryGithubListRepositoriesArgs = {
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
  search?: InputMaybe<Scalars['String']['input']>;
};


export type QueryIntegrationArgs = {
  id: Scalars['String']['input'];
};


export type QueryIntegrationByProviderArgs = {
  provider: GqlProvider;
};


export type QueryIntegrationsArgs = {
  category?: InputMaybe<GqlIntegrationCategory>;
  featuredOnly?: InputMaybe<Scalars['Boolean']['input']>;
};


export type QueryLinearListIssuesArgs = {
  projectId?: InputMaybe<Scalars['String']['input']>;
  teamId?: InputMaybe<Scalars['String']['input']>;
};


export type QueryLinearListProjectsArgs = {
  teamId?: InputMaybe<Scalars['String']['input']>;
};


export type QueryOrganizationArgs = {
  username: Scalars['String']['input'];
};


export type QueryPropertiesArgs = {
  orgUsername: Scalars['String']['input'];
  repoUsername: Scalars['String']['input'];
};


export type QueryRepoArgs = {
  orgUsername: Scalars['String']['input'];
  repoUsername: Scalars['String']['input'];
};


export type QuerySourceArgs = {
  orgUsername: Scalars['String']['input'];
  repoUsername: Scalars['String']['input'];
  sourceId: Scalars['String']['input'];
};


export type QuerySyncOperationArgs = {
  id: Scalars['String']['input'];
};


export type QuerySyncOperationsArgs = {
  endpointId: Scalars['String']['input'];
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
};


export type QueryWebhookEndpointArgs = {
  id: Scalars['String']['input'];
};


export type QueryWebhookEndpointsArgs = {
  provider?: InputMaybe<GqlProvider>;
  repositoryId?: InputMaybe<Scalars['String']['input']>;
  tenantId: Scalars['String']['input'];
};


export type QueryWebhookEventArgs = {
  id: Scalars['String']['input'];
};


export type QueryWebhookEventsArgs = {
  endpointId: Scalars['String']['input'];
  limit?: Scalars['Int']['input'];
  offset?: Scalars['Int']['input'];
};

export type RelationType = {
  __typename?: 'RelationType';
  databaseId: Scalars['String']['output'];
};

export type RelationValue = {
  __typename?: 'RelationValue';
  dataIds: Array<Scalars['String']['output']>;
  databaseId: Scalars['String']['output'];
};

/** Input for removing a user from a repository */
export type RemoveRepoMemberInput = {
  /** Repository ID */
  repoId: Scalars['String']['input'];
  /** User ID to remove */
  userId: Scalars['String']['input'];
};

export type Repo = {
  __typename?: 'Repo';
  dataList: DataList;
  databases: Array<Scalars['String']['output']>;
  description?: Maybe<Scalars['String']['output']>;
  id: Scalars['String']['output'];
  /** Whether the repository is public */
  isPublic: Scalars['Boolean']['output'];
  /**
   * Get members who have access to this repository
   *
   * Returns users with resource-based policies scoped to this repository,
   * including org owners who have implicit owner access.
   */
  members: Array<RepoMember>;
  name: Scalars['String']['output'];
  orgUsername: Scalars['String']['output'];
  organizationId: Scalars['String']['output'];
  /**
   * Get policies for this repository
   *
   * Returns resource-based policies scoped to this repository.
   *
   * For authenticated requests the handler injects a
   * request-scoped `AuthApp` carrying the caller's JWT,
   * so we build a fresh `GetRepoPolicies` with it instead
   * of using the schema-level one (which may hold a
   * placeholder token that production tachyon-api rejects).
   */
  policies: Array<RepoPolicy>;
  properties: Array<Property>;
  sources: Array<Source>;
  tags: Array<Scalars['String']['output']>;
  username: Scalars['String']['output'];
};


export type RepoDataListArgs = {
  page?: InputMaybe<Scalars['Int']['input']>;
  pageSize?: InputMaybe<Scalars['Int']['input']>;
};

/**
 * Repository member with resource-based access control
 *
 * Represents a user who has been granted access to a repository
 * via a scoped policy assignment.
 */
export type RepoMember = {
  __typename?: 'RepoMember';
  /** When the policy was assigned */
  assignedAt: Scalars['DateTime']['output'];
  /** Source of this permission (Repo or Org level) */
  permissionSource: PermissionSource;
  /** Policy ID that grants access */
  policyId: Scalars['String']['output'];
  /** Policy name (e.g., "LibraryRepoOwnerPolicy", "LibraryRepoWriterPolicy") */
  policyName?: Maybe<Scalars['String']['output']>;
  /** TRN resource scope (e.g., "trn:library:repo:rp_xxx") */
  resourceScope?: Maybe<Scalars['String']['output']>;
  /** User details (if available) */
  user?: Maybe<User>;
  /** User ID */
  userId: Scalars['String']['output'];
};

export type RepoPolicy = {
  __typename?: 'RepoPolicy';
  /** Source of this permission (Repo or Org level) */
  permissionSource: PermissionSource;
  role: Scalars['String']['output'];
  user?: Maybe<User>;
  userId: Scalars['String']['output'];
};

export type SelectItem = {
  __typename?: 'SelectItem';
  id: Scalars['String']['output'];
  key: Scalars['String']['output'];
  name: Scalars['String']['output'];
};

export type SelectType = {
  __typename?: 'SelectType';
  options: Array<SelectItem>;
};

export type SelectValue = {
  __typename?: 'SelectValue';
  optionId: Scalars['String']['output'];
};

/** Output for send test webhook mutation. */
export type SendTestWebhookOutput = {
  __typename?: 'SendTestWebhookOutput';
  /** ID of the created event */
  eventId?: Maybe<Scalars['String']['output']>;
  /** Whether the test was sent successfully */
  success: Scalars['Boolean']['output'];
};

export type ServiceAccount = {
  __typename?: 'ServiceAccount';
  createdAt: Scalars['DateTime']['output'];
  id: Scalars['String']['output'];
  name: Scalars['String']['output'];
  tenantId: Scalars['String']['output'];
};

export type Source = {
  __typename?: 'Source';
  id: Scalars['String']['output'];
  name: Scalars['String']['output'];
  repoId: Scalars['String']['output'];
  url?: Maybe<Scalars['String']['output']>;
};

/** Input for starting initial sync. */
export type StartInitialSyncInput = {
  endpointId: Scalars['String']['input'];
};

export type StringValue = {
  __typename?: 'StringValue';
  string: Scalars['String']['output'];
};

/** Suggested property type and options for a frontmatter key */
export type SuggestedProperty = {
  __typename?: 'SuggestedProperty';
  /** Frontmatter key name */
  key: Scalars['String']['output'];
  /** Whether this should be a Select type (<=5 unique values) */
  suggestSelect: Scalars['Boolean']['output'];
  /** Suggested property type */
  suggestedType: PropertyType;
  /** Unique values found (for Select type suggestion) */
  uniqueValues: Array<Scalars['String']['output']>;
};

/** Result of a sync operation */
export type SyncResult = {
  __typename?: 'SyncResult';
  /** Diff preview (for dry-run) */
  diff?: Maybe<Scalars['String']['output']>;
  /** Result ID (commit SHA, etc.) */
  resultId?: Maybe<Scalars['String']['output']>;
  /** Sync status after operation */
  status: SyncStatus;
  /** Whether the sync succeeded */
  success: Scalars['Boolean']['output'];
  /** URL to the synced resource */
  url?: Maybe<Scalars['String']['output']>;
};

/** Sync status enum */
export enum SyncStatus {
  /** Synchronization failed */
  Failed = 'FAILED',
  /** Never synchronized */
  NeverSynced = 'NEVER_SYNCED',
  /** Synchronization pending */
  Pending = 'PENDING',
  /** Successfully synchronized */
  Synced = 'SYNCED'
}

export type SyncToGitHubInput = {
  /** Custom commit message */
  commitMessage?: InputMaybe<Scalars['String']['input']>;
  /** Data ID to sync */
  dataId: Scalars['String']['input'];
  /** If true, only calculate diff without syncing */
  dryRun?: InputMaybe<Scalars['Boolean']['input']>;
  /** Organization username */
  orgUsername: Scalars['String']['input'];
  /** Repository username */
  repoUsername: Scalars['String']['input'];
  /** Target branch (defaults to "main") */
  targetBranch?: InputMaybe<Scalars['String']['input']>;
  /** Target path in the repository */
  targetPath: Scalars['String']['input'];
  /** Target GitHub repository (owner/repo format) */
  targetRepo: Scalars['String']['input'];
};

/** Input for triggering on-demand sync. */
export type TriggerSyncInput = {
  endpointId: Scalars['String']['input'];
  externalIds?: InputMaybe<Array<Scalars['String']['input']>>;
};

export type UpdateDataInputData = {
  /** user_id */
  actor: Scalars['String']['input'];
  dataId: Scalars['String']['input'];
  dataName: Scalars['String']['input'];
  orgUsername: Scalars['String']['input'];
  propertyData: Array<PropertyDataInputData>;
  repoUsername: Scalars['String']['input'];
};

/** Input for updating webhook endpoint configuration. */
export type UpdateEndpointConfigInput = {
  /** Provider-specific configuration as JSON string */
  config: Scalars['String']['input'];
  endpointId: Scalars['String']['input'];
};

/** Input for updating webhook endpoint events. */
export type UpdateEndpointEventsInput = {
  endpointId: Scalars['String']['input'];
  events: Array<Scalars['String']['input']>;
};

/** Input for updating webhook endpoint mapping. */
export type UpdateEndpointMappingInput = {
  endpointId: Scalars['String']['input'];
  /** Property mapping as JSON string (null to remove mapping) */
  mapping?: InputMaybe<Scalars['String']['input']>;
};

/** Input for updating webhook endpoint status. */
export type UpdateEndpointStatusInput = {
  endpointId: Scalars['String']['input'];
  status: GqlEndpointStatus;
};

export type UpdateOrganizationInput = {
  description?: InputMaybe<Scalars['String']['input']>;
  name: Scalars['String']['input'];
  username: Scalars['String']['input'];
  website?: InputMaybe<Scalars['String']['input']>;
};

export type UpdateRepoInput = {
  /**
   * Description of the repository
   * not allow empty string, but can be null
   */
  description?: InputMaybe<Scalars['String']['input']>;
  /** Whether the repository is public */
  isPublic?: InputMaybe<Scalars['Boolean']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  orgUsername: Scalars['String']['input'];
  repoUsername: Scalars['String']['input'];
  /** Tags associated with the repository */
  tags?: InputMaybe<Array<Scalars['String']['input']>>;
};

export type UpdateSourceInput = {
  name?: InputMaybe<Scalars['String']['input']>;
  orgUsername: Scalars['String']['input'];
  repoUsername: Scalars['String']['input'];
  sourceId: Scalars['String']['input'];
  url?: InputMaybe<Scalars['String']['input']>;
};

export type User = {
  __typename?: 'User';
  createdAt: Scalars['DateTime']['output'];
  email?: Maybe<Scalars['String']['output']>;
  emailVerified?: Maybe<Scalars['DateTime']['output']>;
  id: Scalars['String']['output'];
  image?: Maybe<Scalars['String']['output']>;
  name?: Maybe<Scalars['String']['output']>;
  /** Resolve operator organizations accessible to this user */
  organizations: Array<Operator>;
  role: DefaultRole;
  tenantIdList: Array<Scalars['String']['output']>;
  updatedAt: Scalars['DateTime']['output'];
  username?: Maybe<Scalars['String']['output']>;
};

export type VerifyMutationVariables = Exact<{
  token: Scalars['String']['input'];
}>;


export type VerifyMutation = { __typename?: 'Mutation', verify: { __typename?: 'User', id: string, email?: string | null, role: DefaultRole, createdAt: any, updatedAt: any } };

export type SignInOrSignUpMutationVariables = Exact<{
  platformId: Scalars['String']['input'];
  accessToken: Scalars['String']['input'];
  allowSignUp?: InputMaybe<Scalars['Boolean']['input']>;
}>;


export type SignInOrSignUpMutation = { __typename?: 'Mutation', signIn: { __typename?: 'User', id: string, email?: string | null, role: DefaultRole, createdAt: any, updatedAt: any } };

export type MeQueryVariables = Exact<{ [key: string]: never; }>;


export type MeQuery = { __typename?: 'Query', me: { __typename?: 'User', id: string, email?: string | null, role: DefaultRole, createdAt: any, updatedAt: any } };

export type CreateOperatorMutationVariables = Exact<{
  input: CreateOperatorInput;
}>;


export type CreateOperatorMutation = { __typename?: 'Mutation', createOperator: { __typename?: 'Operator', id: string, operatorName: string } };

export type DashboardQueryVariables = Exact<{ [key: string]: never; }>;


export type DashboardQuery = { __typename?: 'Query', me: { __typename?: 'User', name?: string | null, tenantIdList: Array<string>, organizations: Array<{ __typename?: 'Operator', id: string, operatorName: string, platformTenantId: string }> } };

export type DashboardOrgReposQueryVariables = Exact<{
  username: Scalars['String']['input'];
}>;


export type DashboardOrgReposQuery = { __typename?: 'Query', organization: { __typename?: 'Organization', id: string, username: string, repos: Array<{ __typename?: 'Repo', id: string, name: string, username: string, description?: string | null, isPublic: boolean }> } };

export type MeOnDashboardFragment = { __typename?: 'User', name?: string | null, tenantIdList: Array<string>, organizations: Array<{ __typename?: 'Operator', id: string, operatorName: string, platformTenantId: string }> };

export type OrganizationListItemFragment = { __typename?: 'Operator', id: string, operatorName: string, platformTenantId: string };

export type RepoItemOnDashboardFragment = { __typename?: 'Repo', id: string, name: string, username: string, description?: string | null, isPublic: boolean };

export type ErrTestQueryVariables = Exact<{ [key: string]: never; }>;


export type ErrTestQuery = { __typename?: 'Query', errTest: string };

export type NewRepoPageQueryVariables = Exact<{ [key: string]: never; }>;


export type NewRepoPageQuery = { __typename?: 'Query', me: { __typename?: 'User', tenantIdList: Array<string>, organizations: Array<{ __typename?: 'Operator', id: string, operatorName: string }> } };

export type OrganizationOptionFragment = { __typename?: 'Operator', id: string, operatorName: string };

export type DataDetailPageQueryVariables = Exact<{
  orgUsername: Scalars['String']['input'];
  repoUsername: Scalars['String']['input'];
  dataId: Scalars['String']['input'];
}>;


export type DataDetailPageQuery = { __typename?: 'Query', data: { __typename?: 'Data', id: string, name: string, propertyData: Array<{ __typename?: 'PropertyData', propertyId: string, value: { __typename?: 'DateValue', date: string } | { __typename?: 'HtmlValue', html: string } | { __typename?: 'IdValue', id: string } | { __typename?: 'ImageValue', url: string } | { __typename?: 'IntegerValue', number: string } | { __typename?: 'LocationValue', latitude: number, longitude: number } | { __typename?: 'MarkdownValue', markdown: string } | { __typename?: 'MultiSelectValue', optionIds: Array<string> } | { __typename?: 'RelationValue', databaseId: string, dataIds: Array<string> } | { __typename?: 'SelectValue', optionId: string } | { __typename?: 'StringValue', string: string } }> }, properties: Array<{ __typename?: 'Property', id: string, name: string, typ: PropertyType, meta?: { __typename?: 'IdType' } | { __typename?: 'JsonType', json: string } | { __typename?: 'MultiSelectType', options: Array<{ __typename?: 'SelectItem', id: string, key: string, name: string }> } | { __typename?: 'RelationType', databaseId: string } | { __typename?: 'SelectType', options: Array<{ __typename?: 'SelectItem', id: string, key: string, name: string }> } | null }>, dataList: { __typename?: 'DataList', items: Array<{ __typename?: 'Data', id: string, name: string }> }, repo: { __typename?: 'Repo', policies: Array<{ __typename?: 'RepoPolicy', userId: string, role: string }> } };

export type UpdateDataMutationVariables = Exact<{
  input: UpdateDataInputData;
}>;


export type UpdateDataMutation = { __typename?: 'Mutation', updateData: { __typename?: 'Data', id: string, name: string, propertyData: Array<{ __typename?: 'PropertyData', propertyId: string, value: { __typename?: 'DateValue', date: string } | { __typename?: 'HtmlValue', html: string } | { __typename?: 'IdValue', id: string } | { __typename?: 'ImageValue', url: string } | { __typename?: 'IntegerValue', number: string } | { __typename?: 'LocationValue', latitude: number, longitude: number } | { __typename?: 'MarkdownValue', markdown: string } | { __typename?: 'MultiSelectValue', optionIds: Array<string> } | { __typename?: 'RelationValue', databaseId: string, dataIds: Array<string> } | { __typename?: 'SelectValue', optionId: string } | { __typename?: 'StringValue', string: string } }> } };

export type DataOgpMetaQueryVariables = Exact<{
  orgUsername: Scalars['String']['input'];
  repoUsername: Scalars['String']['input'];
  dataId: Scalars['String']['input'];
}>;


export type DataOgpMetaQuery = { __typename?: 'Query', data: { __typename?: 'Data', id: string, name: string, updatedAt: any, propertyData: Array<{ __typename?: 'PropertyData', propertyId: string, value: { __typename?: 'DateValue' } | { __typename?: 'HtmlValue' } | { __typename?: 'IdValue' } | { __typename?: 'ImageValue' } | { __typename?: 'IntegerValue' } | { __typename?: 'LocationValue' } | { __typename?: 'MarkdownValue' } | { __typename?: 'MultiSelectValue' } | { __typename?: 'RelationValue' } | { __typename?: 'SelectValue' } | { __typename?: 'StringValue', string: string } }> } };

export type NewDataQueryVariables = Exact<{
  orgUsername: Scalars['String']['input'];
  repoUsername: Scalars['String']['input'];
}>;


export type NewDataQuery = { __typename?: 'Query', properties: Array<{ __typename?: 'Property', id: string, name: string, typ: PropertyType, meta?: { __typename?: 'IdType' } | { __typename?: 'JsonType', json: string } | { __typename?: 'MultiSelectType', options: Array<{ __typename?: 'SelectItem', id: string, key: string, name: string }> } | { __typename?: 'RelationType', databaseId: string } | { __typename?: 'SelectType', options: Array<{ __typename?: 'SelectItem', id: string, key: string, name: string }> } | null }>, dataList: { __typename?: 'DataList', items: Array<{ __typename?: 'Data', id: string, name: string }> }, repo: { __typename?: 'Repo', policies: Array<{ __typename?: 'RepoPolicy', userId: string, role: string }> } };

export type AddDataMutationVariables = Exact<{
  input: AddDataInputData;
}>;


export type AddDataMutation = { __typename?: 'Mutation', addData: { __typename?: 'Data', id: string, name: string, propertyData: Array<{ __typename?: 'PropertyData', propertyId: string, value: { __typename?: 'DateValue', date: string } | { __typename?: 'HtmlValue', html: string } | { __typename?: 'IdValue', id: string } | { __typename?: 'ImageValue', url: string } | { __typename?: 'IntegerValue', number: string } | { __typename?: 'LocationValue', latitude: number, longitude: number } | { __typename?: 'MarkdownValue', markdown: string } | { __typename?: 'MultiSelectValue', optionIds: Array<string> } | { __typename?: 'RelationValue', databaseId: string, dataIds: Array<string> } | { __typename?: 'SelectValue', optionId: string } | { __typename?: 'StringValue', string: string } }> } };

export type RepoOgpMetaQueryVariables = Exact<{
  org: Scalars['String']['input'];
  repo: Scalars['String']['input'];
}>;


export type RepoOgpMetaQuery = { __typename?: 'Query', repo: { __typename?: 'Repo', id: string, name: string, description?: string | null, isPublic: boolean, tags: Array<string>, policies: Array<{ __typename?: 'RepoPolicy', userId: string }>, dataList: { __typename?: 'DataList', paginator: { __typename?: 'Paginator', totalItems: number } } } };

export type PropertiesQueryVariables = Exact<{
  orgUsername: Scalars['String']['input'];
  repoUsername: Scalars['String']['input'];
}>;


export type PropertiesQuery = { __typename?: 'Query', properties: Array<{ __typename?: 'Property', id: string, name: string, typ: PropertyType, meta?: { __typename?: 'IdType' } | { __typename?: 'JsonType', json: string } | { __typename?: 'MultiSelectType', options: Array<{ __typename?: 'SelectItem', id: string, key: string, name: string }> } | { __typename?: 'RelationType', databaseId: string } | { __typename?: 'SelectType', options: Array<{ __typename?: 'SelectItem', id: string, key: string, name: string }> } | null }>, repo: { __typename?: 'Repo', policies: Array<{ __typename?: 'RepoPolicy', userId: string, role: string }> } };

export type AddPropertyMutationVariables = Exact<{
  input: PropertyInput;
}>;


export type AddPropertyMutation = { __typename?: 'Mutation', addProperty: { __typename?: 'Property', id: string, name: string, typ: PropertyType, meta?: { __typename?: 'IdType' } | { __typename?: 'JsonType', json: string } | { __typename?: 'MultiSelectType', options: Array<{ __typename?: 'SelectItem', id: string, key: string, name: string }> } | { __typename?: 'RelationType', databaseId: string } | { __typename?: 'SelectType', options: Array<{ __typename?: 'SelectItem', id: string, key: string, name: string }> } | null } };

export type UpdatePropertyMutationVariables = Exact<{
  id: Scalars['String']['input'];
  input: PropertyInput;
}>;


export type UpdatePropertyMutation = { __typename?: 'Mutation', updateProperty: { __typename?: 'Property', id: string, name: string, typ: PropertyType, meta?: { __typename?: 'IdType' } | { __typename?: 'JsonType', json: string } | { __typename?: 'MultiSelectType', options: Array<{ __typename?: 'SelectItem', id: string, key: string, name: string }> } | { __typename?: 'RelationType', databaseId: string } | { __typename?: 'SelectType', options: Array<{ __typename?: 'SelectItem', id: string, key: string, name: string }> } | null } };

export type DeletePropertyMutationVariables = Exact<{
  orgUsername: Scalars['String']['input'];
  repoUsername: Scalars['String']['input'];
  id: Scalars['String']['input'];
}>;


export type DeletePropertyMutation = { __typename?: 'Mutation', deleteProperty: string };

export type RepositoryPageWithTagsQueryVariables = Exact<{
  org: Scalars['String']['input'];
  repo: Scalars['String']['input'];
  page?: InputMaybe<Scalars['Int']['input']>;
  pageSize?: InputMaybe<Scalars['Int']['input']>;
}>;


export type RepositoryPageWithTagsQuery = { __typename?: 'Query', repo: { __typename?: 'Repo', id: string, name: string, description?: string | null, isPublic: boolean, tags: Array<string>, dataList: { __typename?: 'DataList', items: Array<{ __typename?: 'Data', id: string, name: string, createdAt: any, updatedAt: any, propertyData: Array<{ __typename?: 'PropertyData', propertyId: string, value: { __typename?: 'DateValue', date: string } | { __typename?: 'HtmlValue', html: string } | { __typename?: 'IdValue', id: string } | { __typename?: 'ImageValue' } | { __typename?: 'IntegerValue', number: string } | { __typename?: 'LocationValue', latitude: number, longitude: number } | { __typename?: 'MarkdownValue' } | { __typename?: 'MultiSelectValue', optionIds: Array<string> } | { __typename?: 'RelationValue', dataIds: Array<string>, databaseId: string } | { __typename?: 'SelectValue', optionId: string } | { __typename?: 'StringValue', string: string } }> }>, paginator: { __typename?: 'Paginator', currentPage: number, totalItems: number, itemsPerPage: number, totalPages: number } }, properties: Array<{ __typename?: 'Property', id: string, name: string, typ: PropertyType, meta?: { __typename?: 'IdType' } | { __typename?: 'JsonType' } | { __typename?: 'MultiSelectType', options: Array<{ __typename?: 'SelectItem', id: string, key: string, name: string }> } | { __typename?: 'RelationType' } | { __typename?: 'SelectType', options: Array<{ __typename?: 'SelectItem', id: string, key: string, name: string }> } | null }>, sources: Array<{ __typename?: 'Source', id: string, name: string, url?: string | null }>, policies: Array<{ __typename?: 'RepoPolicy', userId: string, role: string, user?: { __typename?: 'User', id: string, username?: string | null, name?: string | null, image?: string | null } | null }> } };

export type RepoFieldOnRepoPageFragment = { __typename?: 'Repo', id: string, name: string, description?: string | null, isPublic: boolean, tags: Array<string>, policies: Array<{ __typename?: 'RepoPolicy', userId: string, role: string, user?: { __typename?: 'User', id: string, username?: string | null, name?: string | null, image?: string | null } | null }> };

export type RepositoryPageQueryVariables = Exact<{
  org: Scalars['String']['input'];
  repo: Scalars['String']['input'];
  page?: InputMaybe<Scalars['Int']['input']>;
  pageSize?: InputMaybe<Scalars['Int']['input']>;
}>;


export type RepositoryPageQuery = { __typename?: 'Query', repo: { __typename?: 'Repo', id: string, name: string, description?: string | null, isPublic: boolean, dataList: { __typename?: 'DataList', items: Array<{ __typename?: 'Data', id: string, name: string, createdAt: any, updatedAt: any, propertyData: Array<{ __typename?: 'PropertyData', propertyId: string, value: { __typename?: 'DateValue', date: string } | { __typename?: 'HtmlValue', html: string } | { __typename?: 'IdValue', id: string } | { __typename?: 'ImageValue' } | { __typename?: 'IntegerValue', number: string } | { __typename?: 'LocationValue', latitude: number, longitude: number } | { __typename?: 'MarkdownValue' } | { __typename?: 'MultiSelectValue', optionIds: Array<string> } | { __typename?: 'RelationValue', dataIds: Array<string>, databaseId: string } | { __typename?: 'SelectValue', optionId: string } | { __typename?: 'StringValue', string: string } }> }>, paginator: { __typename?: 'Paginator', currentPage: number, totalItems: number, itemsPerPage: number, totalPages: number } }, properties: Array<{ __typename?: 'Property', id: string, name: string, typ: PropertyType, meta?: { __typename?: 'IdType' } | { __typename?: 'JsonType' } | { __typename?: 'MultiSelectType', options: Array<{ __typename?: 'SelectItem', id: string, key: string, name: string }> } | { __typename?: 'RelationType' } | { __typename?: 'SelectType', options: Array<{ __typename?: 'SelectItem', id: string, key: string, name: string }> } | null }>, sources: Array<{ __typename?: 'Source', id: string, name: string, url?: string | null }>, policies: Array<{ __typename?: 'RepoPolicy', userId: string, role: string, user?: { __typename?: 'User', id: string, username?: string | null, name?: string | null, image?: string | null } | null }> } };

export type RepoFieldOnRepoPageWithoutTagsFragment = { __typename?: 'Repo', id: string, name: string, description?: string | null, isPublic: boolean, policies: Array<{ __typename?: 'RepoPolicy', userId: string, role: string, user?: { __typename?: 'User', id: string, username?: string | null, name?: string | null, image?: string | null } | null }> };

export type SourceFieldOnRepoPageFragment = { __typename?: 'Source', id: string, name: string, url?: string | null };

export type DataFieldOnRepoPageFragment = { __typename?: 'Data', id: string, name: string, createdAt: any, updatedAt: any, propertyData: Array<{ __typename?: 'PropertyData', propertyId: string, value: { __typename?: 'DateValue', date: string } | { __typename?: 'HtmlValue', html: string } | { __typename?: 'IdValue', id: string } | { __typename?: 'ImageValue' } | { __typename?: 'IntegerValue', number: string } | { __typename?: 'LocationValue', latitude: number, longitude: number } | { __typename?: 'MarkdownValue' } | { __typename?: 'MultiSelectValue', optionIds: Array<string> } | { __typename?: 'RelationValue', dataIds: Array<string>, databaseId: string } | { __typename?: 'SelectValue', optionId: string } | { __typename?: 'StringValue', string: string } }> };

export type PaginationFieldFragment = { __typename?: 'Paginator', currentPage: number, totalItems: number, itemsPerPage: number, totalPages: number };

export type PropertyFieldOnRepoPageFragment = { __typename?: 'Property', id: string, name: string, typ: PropertyType, meta?: { __typename?: 'IdType' } | { __typename?: 'JsonType' } | { __typename?: 'MultiSelectType', options: Array<{ __typename?: 'SelectItem', id: string, key: string, name: string }> } | { __typename?: 'RelationType' } | { __typename?: 'SelectType', options: Array<{ __typename?: 'SelectItem', id: string, key: string, name: string }> } | null };

export type InviteRepoMemberMutationVariables = Exact<{
  input: InviteRepoMemberInput;
}>;


export type InviteRepoMemberMutation = { __typename?: 'Mutation', inviteRepoMember: boolean };

export type RemoveRepoMemberMutationVariables = Exact<{
  input: RemoveRepoMemberInput;
}>;


export type RemoveRepoMemberMutation = { __typename?: 'Mutation', removeRepoMember: boolean };

export type ChangeRepoMemberRoleMutationVariables = Exact<{
  input: ChangeRepoMemberRoleInput;
}>;


export type ChangeRepoMemberRoleMutation = { __typename?: 'Mutation', changeRepoMemberRole: boolean };

export type GetRepoMembersQueryVariables = Exact<{
  orgUsername: Scalars['String']['input'];
  repoUsername: Scalars['String']['input'];
}>;


export type GetRepoMembersQuery = { __typename?: 'Query', repo: { __typename?: 'Repo', id: string, members: Array<{ __typename?: 'RepoMember', userId: string, policyId: string, policyName?: string | null, resourceScope?: string | null, assignedAt: any, permissionSource: PermissionSource, user?: { __typename?: 'User', id: string, name?: string | null, email?: string | null, image?: string | null } | null }> } };

export type RepoMemberFieldsFragment = { __typename?: 'RepoMember', userId: string, policyId: string, policyName?: string | null, resourceScope?: string | null, assignedAt: any, permissionSource: PermissionSource, user?: { __typename?: 'User', id: string, name?: string | null, email?: string | null, image?: string | null } | null };

export type GetRepoSettingsQueryVariables = Exact<{
  orgUsername: Scalars['String']['input'];
  repoUsername: Scalars['String']['input'];
}>;


export type GetRepoSettingsQuery = { __typename?: 'Query', repo: { __typename?: 'Repo', id: string } };

export type GetOrgSettingsQueryVariables = Exact<{
  orgUsername: Scalars['String']['input'];
}>;


export type GetOrgSettingsQuery = { __typename?: 'Query', organization: { __typename?: 'Organization', id: string } };

export type UpdateRepoSettingsMutationVariables = Exact<{
  input: UpdateRepoInput;
}>;


export type UpdateRepoSettingsMutation = { __typename?: 'Mutation', updateRepo: { __typename?: 'Repo', id: string, name: string, username: string, description?: string | null, isPublic: boolean, tags: Array<string> } };

export type DeleteRepoMutationVariables = Exact<{
  orgUsername: Scalars['String']['input'];
  repoUsername: Scalars['String']['input'];
}>;


export type DeleteRepoMutation = { __typename?: 'Mutation', deleteRepo: string };

export type ChangeRepoUsernameMutationVariables = Exact<{
  input: ChangeRepoUsernameInput;
}>;


export type ChangeRepoUsernameMutation = { __typename?: 'Mutation', changeRepoUsername: { __typename?: 'Repo', id: string, name: string, username: string, description?: string | null, isPublic: boolean, tags: Array<string> } };

export type GetRepoSettingsPageQueryVariables = Exact<{
  orgUsername: Scalars['String']['input'];
  repoUsername: Scalars['String']['input'];
}>;


export type GetRepoSettingsPageQuery = { __typename?: 'Query', repo: { __typename?: 'Repo', id: string, name: string, username: string, description?: string | null, isPublic: boolean, tags: Array<string>, policies: Array<{ __typename?: 'RepoPolicy', userId: string, role: string }> }, properties: Array<{ __typename?: 'Property', id: string, name: string, typ: PropertyType }> };

export type RepoFieldOnRepoSettingsPageFragment = { __typename?: 'Repo', id: string, name: string, username: string, description?: string | null, isPublic: boolean, tags: Array<string> };

export type PropertyForSettingsPageFragment = { __typename?: 'Property', id: string, name: string, typ: PropertyType };

export type EnableGitHubSyncMutationVariables = Exact<{
  input: EnableGitHubSyncInput;
}>;


export type EnableGitHubSyncMutation = { __typename?: 'Mutation', enableGithubSync: { __typename?: 'EnableGitHubSyncResult', success: boolean, propertyId: string } };

export type DisableGitHubSyncMutationVariables = Exact<{
  input: DisableGitHubSyncInput;
}>;


export type DisableGitHubSyncMutation = { __typename?: 'Mutation', disableGithubSync: { __typename?: 'DisableGitHubSyncResult', success: boolean, deleted: boolean } };

export type EnableLinearSyncMutationVariables = Exact<{
  input: EnableLinearSyncInput;
}>;


export type EnableLinearSyncMutation = { __typename?: 'Mutation', enableLinearSync: { __typename?: 'EnableLinearSyncResult', success: boolean, propertyId: string } };

export type CreateApiKeyMutationVariables = Exact<{
  input: CreateApiKeyInput;
}>;


export type CreateApiKeyMutation = { __typename?: 'Mutation', createApiKey: { __typename?: 'ApiKeyResponse', apiKey: { __typename?: 'PublicApiKey', id: string, value: string, name: string }, serviceAccount: { __typename?: 'ServiceAccount', id: string } } };

export type GetApiKeysQueryVariables = Exact<{
  orgUsername: Scalars['String']['input'];
}>;


export type GetApiKeysQuery = { __typename?: 'Query', apiKeys: Array<{ __typename?: 'PublicApiKey', id: string, name: string, createdAt: any }> };

export type ApiKeyItemFragment = { __typename?: 'PublicApiKey', id: string, name: string, createdAt: any };

export type GitHubListDirectoryContentsQueryVariables = Exact<{
  input: ListGitHubDirectoryInput;
}>;


export type GitHubListDirectoryContentsQuery = { __typename?: 'Query', githubListDirectoryContents: { __typename?: 'GitHubDirectoryContents', truncated: boolean, files: Array<{ __typename?: 'GitHubFileInfo', name: string, path: string, sha: string, size: number, fileType: string, htmlUrl?: string | null }> } };

export type GitHubGetMarkdownPreviewsQueryVariables = Exact<{
  input: GetMarkdownPreviewsInput;
}>;


export type GitHubGetMarkdownPreviewsQuery = { __typename?: 'Query', githubGetMarkdownPreviews: Array<{ __typename?: 'MarkdownImportPreview', path: string, sha: string, frontmatterJson?: string | null, frontmatterKeys: Array<string>, suggestedName: string, bodyPreview: string, parseError?: string | null }> };

export type GitHubAnalyzeFrontmatterQueryVariables = Exact<{
  input: GetMarkdownPreviewsInput;
}>;


export type GitHubAnalyzeFrontmatterQuery = { __typename?: 'Query', githubAnalyzeFrontmatter: { __typename?: 'FrontmatterAnalysis', totalFiles: number, validFiles: number, properties: Array<{ __typename?: 'SuggestedProperty', key: string, suggestedType: PropertyType, uniqueValues: Array<string>, suggestSelect: boolean }> } };

export type ImportMarkdownFromGitHubMutationVariables = Exact<{
  input: ImportMarkdownFromGitHubInput;
}>;


export type ImportMarkdownFromGitHubMutation = { __typename?: 'Mutation', importMarkdownFromGithub: { __typename?: 'ImportMarkdownResult', importedCount: number, updatedCount: number, skippedCount: number, dataIds: Array<string>, repoId: string, errors: Array<{ __typename?: 'ImportError', path: string, message: string }> } };

export type OrganizationFormFragment = { __typename?: 'Organization', name: string, username: string, description?: string | null, website?: string | null };

export type NewDatabaseQueryVariables = Exact<{ [key: string]: never; }>;


export type NewDatabaseQuery = { __typename?: 'Query', me: { __typename?: 'User', tenantIdList: Array<string>, organizations: Array<{ __typename?: 'Operator', id: string, operatorName: string }> } };

export type CreateRepoOnOrgNewDatabasePageMutationVariables = Exact<{
  input: CreateRepoInput;
}>;


export type CreateRepoOnOrgNewDatabasePageMutation = { __typename?: 'Mutation', createRepo: { __typename?: 'Repo', id: string, username: string, databases: Array<string> } };

export type OrgOgpMetaQueryVariables = Exact<{
  username: Scalars['String']['input'];
}>;


export type OrgOgpMetaQuery = { __typename?: 'Query', organization: { __typename?: 'Organization', id: string, name: string, username: string, description?: string | null, repos: Array<{ __typename?: 'Repo', id: string }>, users: Array<{ __typename?: 'User', id: string }> } };

export type OrgPageQueryVariables = Exact<{
  username: Scalars['String']['input'];
}>;


export type OrgPageQuery = { __typename?: 'Query', organization: { __typename?: 'Organization', name: string, username: string, description?: string | null, website?: string | null, id: string, repos: Array<{ __typename?: 'Repo', id: string, name: string, username: string, description?: string | null, isPublic: boolean }>, users: Array<{ __typename?: 'User', id: string, name?: string | null, image?: string | null, email?: string | null, role: DefaultRole }> } };

export type UpdateOrgOnFormMutationVariables = Exact<{
  input: UpdateOrganizationInput;
}>;


export type UpdateOrgOnFormMutation = { __typename?: 'Mutation', updateOrganization: { __typename?: 'Organization', id: string } };

export type DatabaseOnOrgFragment = { __typename?: 'Organization', id: string, name: string, username: string, description?: string | null, website?: string | null, repos: Array<{ __typename?: 'Repo', id: string, name: string, username: string, description?: string | null, isPublic: boolean }>, users: Array<{ __typename?: 'User', id: string, name?: string | null, image?: string | null, email?: string | null, role: DefaultRole }> };

export type RepoItemOnOrgPageFragment = { __typename?: 'Repo', id: string, name: string, username: string, description?: string | null, isPublic: boolean };

export type UserOnOrgPageFragment = { __typename?: 'User', id: string, name?: string | null, image?: string | null, email?: string | null, role: DefaultRole };

export type OrgInvitePageQueryVariables = Exact<{
  orgUsername: Scalars['String']['input'];
}>;


export type OrgInvitePageQuery = { __typename?: 'Query', organization: { __typename?: 'Organization', id: string } };

export type InviteUserMutationVariables = Exact<{
  platformId?: InputMaybe<Scalars['String']['input']>;
  tenantId: Scalars['String']['input'];
  invitee: IdOrEmail;
  notifyUser?: InputMaybe<Scalars['Boolean']['input']>;
}>;


export type InviteUserMutation = { __typename?: 'Mutation', inviteUser: { __typename?: 'User', id: string, email?: string | null, name?: string | null } };

export type GitHubAuthUrlMutationVariables = Exact<{
  state: Scalars['String']['input'];
}>;


export type GitHubAuthUrlMutation = { __typename?: 'Mutation', githubAuthUrl: { __typename?: 'GitHubAuthUrl', url: string, state: string } };

export type GitHubExchangeTokenMutationVariables = Exact<{
  code: Scalars['String']['input'];
  state: Scalars['String']['input'];
}>;


export type GitHubExchangeTokenMutation = { __typename?: 'Mutation', githubExchangeToken: { __typename?: 'GitHubConnection', connected: boolean, username?: string | null, connectedAt?: any | null, expiresAt?: any | null } };

export type GitHubDisconnectMutationVariables = Exact<{ [key: string]: never; }>;


export type GitHubDisconnectMutation = { __typename?: 'Mutation', githubDisconnect: boolean };

export type GitHubConnectionQueryVariables = Exact<{ [key: string]: never; }>;


export type GitHubConnectionQuery = { __typename?: 'Query', githubConnection: { __typename?: 'GitHubConnection', connected: boolean, username?: string | null, connectedAt?: any | null, expiresAt?: any | null } };

export type GitHubListRepositoriesQueryVariables = Exact<{
  search?: InputMaybe<Scalars['String']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
  page?: InputMaybe<Scalars['Int']['input']>;
}>;


export type GitHubListRepositoriesQuery = { __typename?: 'Query', githubListRepositories: Array<{ __typename?: 'GitHubRepository', id: string, name: string, fullName: string, description?: string | null, private: boolean, htmlUrl: string, defaultBranch?: string | null }> };

export type SyncDataToGithubMutationVariables = Exact<{
  input: SyncToGitHubInput;
}>;


export type SyncDataToGithubMutation = { __typename?: 'Mutation', syncDataToGithub: { __typename?: 'SyncResult', success: boolean, status: SyncStatus, resultId?: string | null, url?: string | null, diff?: string | null } };

export type DataForDataDetailFragment = { __typename?: 'Data', id: string, name: string, propertyData: Array<{ __typename?: 'PropertyData', propertyId: string, value: { __typename?: 'DateValue', date: string } | { __typename?: 'HtmlValue', html: string } | { __typename?: 'IdValue', id: string } | { __typename?: 'ImageValue', url: string } | { __typename?: 'IntegerValue', number: string } | { __typename?: 'LocationValue', latitude: number, longitude: number } | { __typename?: 'MarkdownValue', markdown: string } | { __typename?: 'MultiSelectValue', optionIds: Array<string> } | { __typename?: 'RelationValue', databaseId: string, dataIds: Array<string> } | { __typename?: 'SelectValue', optionId: string } | { __typename?: 'StringValue', string: string } }> };

export type DataListForDataListCardFragment = { __typename?: 'DataList', items: Array<{ __typename?: 'Data', id: string, name: string }> };

export type DataCountForBulkSyncQueryVariables = Exact<{
  org: Scalars['String']['input'];
  repo: Scalars['String']['input'];
}>;


export type DataCountForBulkSyncQuery = { __typename?: 'Query', repo: { __typename?: 'Repo', dataList: { __typename?: 'DataList', paginator: { __typename?: 'Paginator', totalItems: number } } } };

export type BulkSyncExtGithubMutationVariables = Exact<{
  input: BulkSyncExtGithubInput;
}>;


export type BulkSyncExtGithubMutation = { __typename?: 'Mutation', bulkSyncExtGithub: { __typename?: 'BulkSyncExtGithubResult', updatedCount: number, skippedCount: number, totalCount: number } };

export type PropertyForPropertiesUiFragment = { __typename?: 'Property', id: string, name: string, typ: PropertyType, meta?: { __typename?: 'IdType' } | { __typename?: 'JsonType', json: string } | { __typename?: 'MultiSelectType', options: Array<{ __typename?: 'SelectItem', id: string, key: string, name: string }> } | { __typename?: 'RelationType', databaseId: string } | { __typename?: 'SelectType', options: Array<{ __typename?: 'SelectItem', id: string, key: string, name: string }> } | null };

type PropertyTypeMetaForPropertiesUi_IdType_Fragment = { __typename?: 'IdType' };

type PropertyTypeMetaForPropertiesUi_JsonType_Fragment = { __typename?: 'JsonType', json: string };

type PropertyTypeMetaForPropertiesUi_MultiSelectType_Fragment = { __typename?: 'MultiSelectType', options: Array<{ __typename?: 'SelectItem', id: string, key: string, name: string }> };

type PropertyTypeMetaForPropertiesUi_RelationType_Fragment = { __typename?: 'RelationType', databaseId: string };

type PropertyTypeMetaForPropertiesUi_SelectType_Fragment = { __typename?: 'SelectType', options: Array<{ __typename?: 'SelectItem', id: string, key: string, name: string }> };

export type PropertyTypeMetaForPropertiesUiFragment = PropertyTypeMetaForPropertiesUi_IdType_Fragment | PropertyTypeMetaForPropertiesUi_JsonType_Fragment | PropertyTypeMetaForPropertiesUi_MultiSelectType_Fragment | PropertyTypeMetaForPropertiesUi_RelationType_Fragment | PropertyTypeMetaForPropertiesUi_SelectType_Fragment;

export type JsonTypeMetaForPropertiesUiFragment = { __typename?: 'JsonType', json: string };

export type RelationTypeMetaForPropertiesUiFragment = { __typename?: 'RelationType', databaseId: string };

export type SelectTypeMetaForPropertiesUiFragment = { __typename?: 'SelectType', options: Array<{ __typename?: 'SelectItem', id: string, key: string, name: string }> };

export type MultiSelectTypeMetaForPropertiesUiFragment = { __typename?: 'MultiSelectType', options: Array<{ __typename?: 'SelectItem', id: string, key: string, name: string }> };

export type PropertyDataForEditorFragment = { __typename?: 'PropertyData', propertyId: string, value: { __typename?: 'DateValue', date: string } | { __typename?: 'HtmlValue', html: string } | { __typename?: 'IdValue', id: string } | { __typename?: 'ImageValue', url: string } | { __typename?: 'IntegerValue', number: string } | { __typename?: 'LocationValue', latitude: number, longitude: number } | { __typename?: 'MarkdownValue', markdown: string } | { __typename?: 'MultiSelectValue', optionIds: Array<string> } | { __typename?: 'RelationValue', databaseId: string, dataIds: Array<string> } | { __typename?: 'SelectValue', optionId: string } | { __typename?: 'StringValue', string: string } };

export type IdValueForEditorFragment = { __typename?: 'IdValue', id: string };

export type StringValueForEditorFragment = { __typename?: 'StringValue', string: string };

export type IntegerValueForEditorFragment = { __typename?: 'IntegerValue', number: string };

export type HtmlValueForEditorFragment = { __typename?: 'HtmlValue', html: string };

export type MarkdownValueForEditorFragment = { __typename?: 'MarkdownValue', markdown: string };

export type RelationValueForEditorFragment = { __typename?: 'RelationValue', databaseId: string, dataIds: Array<string> };

export type SelectValueForEditorFragment = { __typename?: 'SelectValue', optionId: string };

export type MultiSelectValueForEditorFragment = { __typename?: 'MultiSelectValue', optionIds: Array<string> };

export type LocationValueForEditorFragment = { __typename?: 'LocationValue', latitude: number, longitude: number };

export type DateValueForEditorFragment = { __typename?: 'DateValue', date: string };

export type ImageValueForEditorFragment = { __typename?: 'ImageValue', url: string };

export type PropertyForEditorFragment = { __typename?: 'Property', id: string, name: string, typ: PropertyType, meta?: { __typename?: 'IdType' } | { __typename?: 'JsonType', json: string } | { __typename?: 'MultiSelectType', options: Array<{ __typename?: 'SelectItem', id: string, key: string, name: string }> } | { __typename?: 'RelationType', databaseId: string } | { __typename?: 'SelectType', options: Array<{ __typename?: 'SelectItem', id: string, key: string, name: string }> } | null };

type PropertyTypeMetaForEditor_IdType_Fragment = { __typename?: 'IdType' };

type PropertyTypeMetaForEditor_JsonType_Fragment = { __typename?: 'JsonType', json: string };

type PropertyTypeMetaForEditor_MultiSelectType_Fragment = { __typename?: 'MultiSelectType', options: Array<{ __typename?: 'SelectItem', id: string, key: string, name: string }> };

type PropertyTypeMetaForEditor_RelationType_Fragment = { __typename?: 'RelationType', databaseId: string };

type PropertyTypeMetaForEditor_SelectType_Fragment = { __typename?: 'SelectType', options: Array<{ __typename?: 'SelectItem', id: string, key: string, name: string }> };

export type PropertyTypeMetaForEditorFragment = PropertyTypeMetaForEditor_IdType_Fragment | PropertyTypeMetaForEditor_JsonType_Fragment | PropertyTypeMetaForEditor_MultiSelectType_Fragment | PropertyTypeMetaForEditor_RelationType_Fragment | PropertyTypeMetaForEditor_SelectType_Fragment;

export type JsonTypeMetaForEditorFragment = { __typename?: 'JsonType', json: string };

export type RelationTypeMetaForEditorFragment = { __typename?: 'RelationType', databaseId: string };

export type SelectTypeMetaForEditorFragment = { __typename?: 'SelectType', options: Array<{ __typename?: 'SelectItem', id: string, key: string, name: string }> };

export type MultiSelectTypeMetaForEditorFragment = { __typename?: 'MultiSelectType', options: Array<{ __typename?: 'SelectItem', id: string, key: string, name: string }> };

export type CreateOrganizationMutationVariables = Exact<{
  input: CreateOrganizationInput;
}>;


export type CreateOrganizationMutation = { __typename?: 'Mutation', createOrganization: { __typename?: 'Organization', id: string, name: string, username: string, description?: string | null } };

export type WritePermissionHooksPolicyFieldsFragment = { __typename?: 'RepoPolicy', role: string, userId: string };

export const OrganizationListItemFragmentDoc = gql`
    fragment organizationListItem on Operator {
  id
  operatorName
  platformTenantId
}
    `;
export const MeOnDashboardFragmentDoc = gql`
    fragment meOnDashboard on User {
  name
  tenantIdList
  organizations {
    ...organizationListItem
  }
}
    ${OrganizationListItemFragmentDoc}`;
export const RepoItemOnDashboardFragmentDoc = gql`
    fragment repoItemOnDashboard on Repo {
  id
  name
  username
  description
  isPublic
}
    `;
export const OrganizationOptionFragmentDoc = gql`
    fragment organizationOption on Operator {
  id
  operatorName
}
    `;
export const RepoFieldOnRepoPageFragmentDoc = gql`
    fragment RepoFieldOnRepoPage on Repo {
  id
  name
  description
  isPublic
  tags
  policies {
    userId
    role
    user {
      id
      username
      name
      image
    }
  }
}
    `;
export const RepoFieldOnRepoPageWithoutTagsFragmentDoc = gql`
    fragment RepoFieldOnRepoPageWithoutTags on Repo {
  id
  name
  description
  isPublic
  policies {
    userId
    role
    user {
      id
      username
      name
      image
    }
  }
}
    `;
export const SourceFieldOnRepoPageFragmentDoc = gql`
    fragment SourceFieldOnRepoPage on Source {
  id
  name
  url
}
    `;
export const DataFieldOnRepoPageFragmentDoc = gql`
    fragment DataFieldOnRepoPage on Data {
  id
  name
  createdAt
  updatedAt
  propertyData {
    propertyId
    value {
      ... on StringValue {
        string
      }
      ... on IntegerValue {
        number
      }
      ... on HtmlValue {
        html
      }
      ... on RelationValue {
        dataIds
        databaseId
      }
      ... on SelectValue {
        optionId
      }
      ... on MultiSelectValue {
        optionIds
      }
      ... on IdValue {
        id
      }
      ... on LocationValue {
        latitude
        longitude
      }
      ... on DateValue {
        date
      }
    }
  }
}
    `;
export const PaginationFieldFragmentDoc = gql`
    fragment PaginationField on Paginator {
  currentPage
  totalItems
  itemsPerPage
  totalPages
}
    `;
export const PropertyFieldOnRepoPageFragmentDoc = gql`
    fragment PropertyFieldOnRepoPage on Property {
  id
  name
  typ
  meta {
    ... on SelectType {
      options {
        id
        key
        name
      }
    }
    ... on MultiSelectType {
      options {
        id
        key
        name
      }
    }
  }
}
    `;
export const RepoMemberFieldsFragmentDoc = gql`
    fragment RepoMemberFields on RepoMember {
  userId
  policyId
  policyName
  resourceScope
  assignedAt
  permissionSource
  user {
    id
    name
    email
    image
  }
}
    `;
export const RepoFieldOnRepoSettingsPageFragmentDoc = gql`
    fragment RepoFieldOnRepoSettingsPage on Repo {
  id
  name
  username
  description
  isPublic
  tags
}
    `;
export const PropertyForSettingsPageFragmentDoc = gql`
    fragment PropertyForSettingsPage on Property {
  id
  name
  typ
}
    `;
export const ApiKeyItemFragmentDoc = gql`
    fragment ApiKeyItem on PublicApiKey {
  id
  name
  createdAt
}
    `;
export const OrganizationFormFragmentDoc = gql`
    fragment OrganizationForm on Organization {
  name
  username
  description
  website
}
    `;
export const RepoItemOnOrgPageFragmentDoc = gql`
    fragment RepoItemOnOrgPage on Repo {
  id
  name
  username
  description
  isPublic
}
    `;
export const UserOnOrgPageFragmentDoc = gql`
    fragment UserOnOrgPage on User {
  id
  name
  image
  email
  role
}
    `;
export const DatabaseOnOrgFragmentDoc = gql`
    fragment DatabaseOnOrg on Organization {
  id
  name
  username
  description
  website
  repos {
    ...RepoItemOnOrgPage
  }
  users {
    ...UserOnOrgPage
  }
}
    ${RepoItemOnOrgPageFragmentDoc}
${UserOnOrgPageFragmentDoc}`;
export const StringValueForEditorFragmentDoc = gql`
    fragment StringValueForEditor on StringValue {
  string
}
    `;
export const IntegerValueForEditorFragmentDoc = gql`
    fragment IntegerValueForEditor on IntegerValue {
  number
}
    `;
export const IdValueForEditorFragmentDoc = gql`
    fragment IdValueForEditor on IdValue {
  id
}
    `;
export const HtmlValueForEditorFragmentDoc = gql`
    fragment HtmlValueForEditor on HtmlValue {
  html
}
    `;
export const MarkdownValueForEditorFragmentDoc = gql`
    fragment MarkdownValueForEditor on MarkdownValue {
  markdown
}
    `;
export const RelationValueForEditorFragmentDoc = gql`
    fragment RelationValueForEditor on RelationValue {
  databaseId
  dataIds
}
    `;
export const SelectValueForEditorFragmentDoc = gql`
    fragment SelectValueForEditor on SelectValue {
  optionId
}
    `;
export const MultiSelectValueForEditorFragmentDoc = gql`
    fragment MultiSelectValueForEditor on MultiSelectValue {
  optionIds
}
    `;
export const LocationValueForEditorFragmentDoc = gql`
    fragment LocationValueForEditor on LocationValue {
  latitude
  longitude
}
    `;
export const DateValueForEditorFragmentDoc = gql`
    fragment DateValueForEditor on DateValue {
  date
}
    `;
export const ImageValueForEditorFragmentDoc = gql`
    fragment ImageValueForEditor on ImageValue {
  url
}
    `;
export const PropertyDataForEditorFragmentDoc = gql`
    fragment PropertyDataForEditor on PropertyData {
  propertyId
  value {
    ...StringValueForEditor
    ...IntegerValueForEditor
    ...IdValueForEditor
    ...HtmlValueForEditor
    ...MarkdownValueForEditor
    ...RelationValueForEditor
    ...SelectValueForEditor
    ...MultiSelectValueForEditor
    ...LocationValueForEditor
    ...DateValueForEditor
    ...ImageValueForEditor
  }
}
    ${StringValueForEditorFragmentDoc}
${IntegerValueForEditorFragmentDoc}
${IdValueForEditorFragmentDoc}
${HtmlValueForEditorFragmentDoc}
${MarkdownValueForEditorFragmentDoc}
${RelationValueForEditorFragmentDoc}
${SelectValueForEditorFragmentDoc}
${MultiSelectValueForEditorFragmentDoc}
${LocationValueForEditorFragmentDoc}
${DateValueForEditorFragmentDoc}
${ImageValueForEditorFragmentDoc}`;
export const DataForDataDetailFragmentDoc = gql`
    fragment DataForDataDetail on Data {
  id
  name
  propertyData {
    ...PropertyDataForEditor
  }
}
    ${PropertyDataForEditorFragmentDoc}`;
export const DataListForDataListCardFragmentDoc = gql`
    fragment DataListForDataListCard on DataList {
  items {
    id
    name
  }
}
    `;
export const RelationTypeMetaForPropertiesUiFragmentDoc = gql`
    fragment RelationTypeMetaForPropertiesUi on RelationType {
  databaseId
}
    `;
export const SelectTypeMetaForPropertiesUiFragmentDoc = gql`
    fragment SelectTypeMetaForPropertiesUi on SelectType {
  options {
    id
    key
    name
  }
}
    `;
export const MultiSelectTypeMetaForPropertiesUiFragmentDoc = gql`
    fragment MultiSelectTypeMetaForPropertiesUi on MultiSelectType {
  options {
    id
    key
    name
  }
}
    `;
export const JsonTypeMetaForPropertiesUiFragmentDoc = gql`
    fragment JsonTypeMetaForPropertiesUi on JsonType {
  json
}
    `;
export const PropertyTypeMetaForPropertiesUiFragmentDoc = gql`
    fragment PropertyTypeMetaForPropertiesUi on PropertyTypeMeta {
  ...RelationTypeMetaForPropertiesUi
  ...SelectTypeMetaForPropertiesUi
  ...MultiSelectTypeMetaForPropertiesUi
  ...JsonTypeMetaForPropertiesUi
}
    ${RelationTypeMetaForPropertiesUiFragmentDoc}
${SelectTypeMetaForPropertiesUiFragmentDoc}
${MultiSelectTypeMetaForPropertiesUiFragmentDoc}
${JsonTypeMetaForPropertiesUiFragmentDoc}`;
export const PropertyForPropertiesUiFragmentDoc = gql`
    fragment PropertyForPropertiesUi on Property {
  id
  name
  typ
  meta {
    ...PropertyTypeMetaForPropertiesUi
  }
}
    ${PropertyTypeMetaForPropertiesUiFragmentDoc}`;
export const RelationTypeMetaForEditorFragmentDoc = gql`
    fragment RelationTypeMetaForEditor on RelationType {
  databaseId
}
    `;
export const SelectTypeMetaForEditorFragmentDoc = gql`
    fragment SelectTypeMetaForEditor on SelectType {
  options {
    id
    key
    name
  }
}
    `;
export const MultiSelectTypeMetaForEditorFragmentDoc = gql`
    fragment MultiSelectTypeMetaForEditor on MultiSelectType {
  options {
    id
    key
    name
  }
}
    `;
export const JsonTypeMetaForEditorFragmentDoc = gql`
    fragment JsonTypeMetaForEditor on JsonType {
  json
}
    `;
export const PropertyTypeMetaForEditorFragmentDoc = gql`
    fragment PropertyTypeMetaForEditor on PropertyTypeMeta {
  ...RelationTypeMetaForEditor
  ...SelectTypeMetaForEditor
  ...MultiSelectTypeMetaForEditor
  ...JsonTypeMetaForEditor
}
    ${RelationTypeMetaForEditorFragmentDoc}
${SelectTypeMetaForEditorFragmentDoc}
${MultiSelectTypeMetaForEditorFragmentDoc}
${JsonTypeMetaForEditorFragmentDoc}`;
export const PropertyForEditorFragmentDoc = gql`
    fragment PropertyForEditor on Property {
  id
  name
  typ
  meta {
    ...PropertyTypeMetaForEditor
  }
}
    ${PropertyTypeMetaForEditorFragmentDoc}`;
export const WritePermissionHooksPolicyFieldsFragmentDoc = gql`
    fragment WritePermissionHooksPolicyFields on RepoPolicy {
  role
  userId
}
    `;
export const VerifyDocument = gql`
    mutation verify($token: String!) {
  verify(token: $token) {
    id
    email
    role
    createdAt
    updatedAt
  }
}
    `;
export const SignInOrSignUpDocument = gql`
    mutation signInOrSignUp($platformId: String!, $accessToken: String!, $allowSignUp: Boolean) {
  signIn(
    platformId: $platformId
    accessToken: $accessToken
    allowSignUp: $allowSignUp
  ) {
    id
    email
    role
    createdAt
    updatedAt
  }
}
    `;
export const MeDocument = gql`
    query me {
  me {
    id
    email
    role
    createdAt
    updatedAt
  }
}
    `;
export const CreateOperatorDocument = gql`
    mutation createOperator($input: CreateOperatorInput!) {
  createOperator(input: $input) {
    id
    operatorName
  }
}
    `;
export const DashboardDocument = gql`
    query dashboard {
  me {
    ...meOnDashboard
  }
}
    ${MeOnDashboardFragmentDoc}`;
export const DashboardOrgReposDocument = gql`
    query dashboardOrgRepos($username: String!) {
  organization(username: $username) {
    id
    username
    repos {
      ...repoItemOnDashboard
    }
  }
}
    ${RepoItemOnDashboardFragmentDoc}`;
export const ErrTestDocument = gql`
    query ErrTest {
  errTest
}
    `;
export const NewRepoPageDocument = gql`
    query newRepoPage {
  me {
    tenantIdList
    organizations {
      ...organizationOption
    }
  }
}
    ${OrganizationOptionFragmentDoc}`;
export const DataDetailPageDocument = gql`
    query dataDetailPage($orgUsername: String!, $repoUsername: String!, $dataId: String!) {
  data(orgUsername: $orgUsername, repoUsername: $repoUsername, dataId: $dataId) {
    ...DataForDataDetail
  }
  properties(orgUsername: $orgUsername, repoUsername: $repoUsername) {
    ...PropertyForEditor
  }
  dataList(orgUsername: $orgUsername, repoUsername: $repoUsername) {
    ...DataListForDataListCard
  }
  repo(orgUsername: $orgUsername, repoUsername: $repoUsername) {
    policies {
      userId
      role
    }
  }
}
    ${DataForDataDetailFragmentDoc}
${PropertyForEditorFragmentDoc}
${DataListForDataListCardFragmentDoc}`;
export const UpdateDataDocument = gql`
    mutation updateData($input: UpdateDataInputData!) {
  updateData(input: $input) {
    ...DataForDataDetail
  }
}
    ${DataForDataDetailFragmentDoc}`;
export const DataOgpMetaDocument = gql`
    query dataOgpMeta($orgUsername: String!, $repoUsername: String!, $dataId: String!) {
  data(orgUsername: $orgUsername, repoUsername: $repoUsername, dataId: $dataId) {
    id
    name
    updatedAt
    propertyData {
      propertyId
      value {
        ... on StringValue {
          string
        }
      }
    }
  }
}
    `;
export const NewDataDocument = gql`
    query newData($orgUsername: String!, $repoUsername: String!) {
  properties(orgUsername: $orgUsername, repoUsername: $repoUsername) {
    ...PropertyForEditor
  }
  dataList(orgUsername: $orgUsername, repoUsername: $repoUsername) {
    ...DataListForDataListCard
  }
  repo(orgUsername: $orgUsername, repoUsername: $repoUsername) {
    policies {
      userId
      role
    }
  }
}
    ${PropertyForEditorFragmentDoc}
${DataListForDataListCardFragmentDoc}`;
export const AddDataDocument = gql`
    mutation addData($input: AddDataInputData!) {
  addData(input: $input) {
    ...DataForDataDetail
  }
}
    ${DataForDataDetailFragmentDoc}`;
export const RepoOgpMetaDocument = gql`
    query repoOgpMeta($org: String!, $repo: String!) {
  repo(orgUsername: $org, repoUsername: $repo) {
    id
    name
    description
    isPublic
    tags
    policies {
      userId
    }
    dataList {
      paginator {
        totalItems
      }
    }
  }
}
    `;
export const PropertiesDocument = gql`
    query properties($orgUsername: String!, $repoUsername: String!) {
  properties(orgUsername: $orgUsername, repoUsername: $repoUsername) {
    ...PropertyForPropertiesUi
  }
  repo(orgUsername: $orgUsername, repoUsername: $repoUsername) {
    policies {
      userId
      role
    }
  }
}
    ${PropertyForPropertiesUiFragmentDoc}`;
export const AddPropertyDocument = gql`
    mutation addProperty($input: PropertyInput!) {
  addProperty(input: $input) {
    ...PropertyForPropertiesUi
  }
}
    ${PropertyForPropertiesUiFragmentDoc}`;
export const UpdatePropertyDocument = gql`
    mutation updateProperty($id: String!, $input: PropertyInput!) {
  updateProperty(id: $id, input: $input) {
    ...PropertyForPropertiesUi
  }
}
    ${PropertyForPropertiesUiFragmentDoc}`;
export const DeletePropertyDocument = gql`
    mutation deleteProperty($orgUsername: String!, $repoUsername: String!, $id: String!) {
  deleteProperty(
    orgUsername: $orgUsername
    repoUsername: $repoUsername
    propertyId: $id
  )
}
    `;
export const RepositoryPageWithTagsDocument = gql`
    query repositoryPageWithTags($org: String!, $repo: String!, $page: Int, $pageSize: Int) {
  repo(orgUsername: $org, repoUsername: $repo) {
    ...RepoFieldOnRepoPage
    dataList(page: $page, pageSize: $pageSize) {
      items {
        ...DataFieldOnRepoPage
      }
      paginator {
        ...PaginationField
      }
    }
    properties {
      ...PropertyFieldOnRepoPage
    }
    sources {
      ...SourceFieldOnRepoPage
    }
  }
}
    ${RepoFieldOnRepoPageFragmentDoc}
${DataFieldOnRepoPageFragmentDoc}
${PaginationFieldFragmentDoc}
${PropertyFieldOnRepoPageFragmentDoc}
${SourceFieldOnRepoPageFragmentDoc}`;
export const RepositoryPageDocument = gql`
    query repositoryPage($org: String!, $repo: String!, $page: Int, $pageSize: Int) {
  repo(orgUsername: $org, repoUsername: $repo) {
    ...RepoFieldOnRepoPageWithoutTags
    dataList(page: $page, pageSize: $pageSize) {
      items {
        ...DataFieldOnRepoPage
      }
      paginator {
        ...PaginationField
      }
    }
    properties {
      ...PropertyFieldOnRepoPage
    }
    sources {
      ...SourceFieldOnRepoPage
    }
  }
}
    ${RepoFieldOnRepoPageWithoutTagsFragmentDoc}
${DataFieldOnRepoPageFragmentDoc}
${PaginationFieldFragmentDoc}
${PropertyFieldOnRepoPageFragmentDoc}
${SourceFieldOnRepoPageFragmentDoc}`;
export const InviteRepoMemberDocument = gql`
    mutation InviteRepoMember($input: InviteRepoMemberInput!) {
  inviteRepoMember(input: $input)
}
    `;
export const RemoveRepoMemberDocument = gql`
    mutation RemoveRepoMember($input: RemoveRepoMemberInput!) {
  removeRepoMember(input: $input)
}
    `;
export const ChangeRepoMemberRoleDocument = gql`
    mutation ChangeRepoMemberRole($input: ChangeRepoMemberRoleInput!) {
  changeRepoMemberRole(input: $input)
}
    `;
export const GetRepoMembersDocument = gql`
    query GetRepoMembers($orgUsername: String!, $repoUsername: String!) {
  repo(orgUsername: $orgUsername, repoUsername: $repoUsername) {
    id
    members {
      userId
      policyId
      policyName
      resourceScope
      assignedAt
      permissionSource
      user {
        id
        name
        email
        image
      }
    }
  }
}
    `;
export const GetRepoSettingsDocument = gql`
    query getRepoSettings($orgUsername: String!, $repoUsername: String!) {
  repo(orgUsername: $orgUsername, repoUsername: $repoUsername) {
    id
  }
}
    `;
export const GetOrgSettingsDocument = gql`
    query GetOrgSettings($orgUsername: String!) {
  organization(username: $orgUsername) {
    id
  }
}
    `;
export const UpdateRepoSettingsDocument = gql`
    mutation UpdateRepoSettings($input: UpdateRepoInput!) {
  updateRepo(input: $input) {
    ...RepoFieldOnRepoSettingsPage
  }
}
    ${RepoFieldOnRepoSettingsPageFragmentDoc}`;
export const DeleteRepoDocument = gql`
    mutation DeleteRepo($orgUsername: String!, $repoUsername: String!) {
  deleteRepo(orgUsername: $orgUsername, repoUsername: $repoUsername)
}
    `;
export const ChangeRepoUsernameDocument = gql`
    mutation ChangeRepoUsername($input: ChangeRepoUsernameInput!) {
  changeRepoUsername(input: $input) {
    ...RepoFieldOnRepoSettingsPage
  }
}
    ${RepoFieldOnRepoSettingsPageFragmentDoc}`;
export const GetRepoSettingsPageDocument = gql`
    query getRepoSettingsPage($orgUsername: String!, $repoUsername: String!) {
  repo(orgUsername: $orgUsername, repoUsername: $repoUsername) {
    ...RepoFieldOnRepoSettingsPage
    policies {
      userId
      role
    }
  }
  properties(orgUsername: $orgUsername, repoUsername: $repoUsername) {
    ...PropertyForSettingsPage
  }
}
    ${RepoFieldOnRepoSettingsPageFragmentDoc}
${PropertyForSettingsPageFragmentDoc}`;
export const EnableGitHubSyncDocument = gql`
    mutation EnableGitHubSync($input: EnableGitHubSyncInput!) {
  enableGithubSync(input: $input) {
    success
    propertyId
  }
}
    `;
export const DisableGitHubSyncDocument = gql`
    mutation DisableGitHubSync($input: DisableGitHubSyncInput!) {
  disableGithubSync(input: $input) {
    success
    deleted
  }
}
    `;
export const EnableLinearSyncDocument = gql`
    mutation EnableLinearSync($input: EnableLinearSyncInput!) {
  enableLinearSync(input: $input) {
    success
    propertyId
  }
}
    `;
export const CreateApiKeyDocument = gql`
    mutation createAPIKey($input: CreateApiKeyInput!) {
  createApiKey(input: $input) {
    apiKey {
      id
      value
      name
    }
    serviceAccount {
      id
    }
  }
}
    `;
export const GetApiKeysDocument = gql`
    query getApiKeys($orgUsername: String!) {
  apiKeys(orgUsername: $orgUsername) {
    id
    name
    createdAt
  }
}
    `;
export const GitHubListDirectoryContentsDocument = gql`
    query GitHubListDirectoryContents($input: ListGitHubDirectoryInput!) {
  githubListDirectoryContents(input: $input) {
    files {
      name
      path
      sha
      size
      fileType
      htmlUrl
    }
    truncated
  }
}
    `;
export const GitHubGetMarkdownPreviewsDocument = gql`
    query GitHubGetMarkdownPreviews($input: GetMarkdownPreviewsInput!) {
  githubGetMarkdownPreviews(input: $input) {
    path
    sha
    frontmatterJson
    frontmatterKeys
    suggestedName
    bodyPreview
    parseError
  }
}
    `;
export const GitHubAnalyzeFrontmatterDocument = gql`
    query GitHubAnalyzeFrontmatter($input: GetMarkdownPreviewsInput!) {
  githubAnalyzeFrontmatter(input: $input) {
    properties {
      key
      suggestedType
      uniqueValues
      suggestSelect
    }
    totalFiles
    validFiles
  }
}
    `;
export const ImportMarkdownFromGitHubDocument = gql`
    mutation ImportMarkdownFromGitHub($input: ImportMarkdownFromGitHubInput!) {
  importMarkdownFromGithub(input: $input) {
    importedCount
    updatedCount
    skippedCount
    errors {
      path
      message
    }
    dataIds
    repoId
  }
}
    `;
export const NewDatabaseDocument = gql`
    query newDatabase {
  me {
    tenantIdList
    organizations {
      ...organizationOption
    }
  }
}
    ${OrganizationOptionFragmentDoc}`;
export const CreateRepoOnOrgNewDatabasePageDocument = gql`
    mutation createRepoOnOrgNewDatabasePage($input: CreateRepoInput!) {
  createRepo(input: $input) {
    id
    username
    databases
  }
}
    `;
export const OrgOgpMetaDocument = gql`
    query orgOgpMeta($username: String!) {
  organization(username: $username) {
    id
    name
    username
    description
    repos {
      id
    }
    users {
      id
    }
  }
}
    `;
export const OrgPageDocument = gql`
    query orgPage($username: String!) {
  organization(username: $username) {
    ...OrganizationForm
    ...DatabaseOnOrg
  }
}
    ${OrganizationFormFragmentDoc}
${DatabaseOnOrgFragmentDoc}`;
export const UpdateOrgOnFormDocument = gql`
    mutation updateOrgOnForm($input: UpdateOrganizationInput!) {
  updateOrganization(input: $input) {
    id
  }
}
    `;
export const OrgInvitePageDocument = gql`
    query orgInvitePage($orgUsername: String!) {
  organization(username: $orgUsername) {
    id
  }
}
    `;
export const InviteUserDocument = gql`
    mutation inviteUser($platformId: String, $tenantId: String!, $invitee: IdOrEmail!, $notifyUser: Boolean) {
  inviteUser(
    platformId: $platformId
    tenantId: $tenantId
    invitee: $invitee
    notifyUser: $notifyUser
  ) {
    id
    email
    name
  }
}
    `;
export const GitHubAuthUrlDocument = gql`
    mutation GitHubAuthUrl($state: String!) {
  githubAuthUrl(state: $state) {
    url
    state
  }
}
    `;
export const GitHubExchangeTokenDocument = gql`
    mutation GitHubExchangeToken($code: String!, $state: String!) {
  githubExchangeToken(code: $code, state: $state) {
    connected
    username
    connectedAt
    expiresAt
  }
}
    `;
export const GitHubDisconnectDocument = gql`
    mutation GitHubDisconnect {
  githubDisconnect
}
    `;
export const GitHubConnectionDocument = gql`
    query GitHubConnection {
  githubConnection {
    connected
    username
    connectedAt
    expiresAt
  }
}
    `;
export const GitHubListRepositoriesDocument = gql`
    query GitHubListRepositories($search: String, $perPage: Int, $page: Int) {
  githubListRepositories(search: $search, perPage: $perPage, page: $page) {
    id
    name
    fullName
    description
    private
    htmlUrl
    defaultBranch
  }
}
    `;
export const SyncDataToGithubDocument = gql`
    mutation SyncDataToGithub($input: SyncToGitHubInput!) {
  syncDataToGithub(input: $input) {
    success
    status
    resultId
    url
    diff
  }
}
    `;
export const DataCountForBulkSyncDocument = gql`
    query DataCountForBulkSync($org: String!, $repo: String!) {
  repo(orgUsername: $org, repoUsername: $repo) {
    dataList(pageSize: 1) {
      paginator {
        totalItems
      }
    }
  }
}
    `;
export const BulkSyncExtGithubDocument = gql`
    mutation BulkSyncExtGithub($input: BulkSyncExtGithubInput!) {
  bulkSyncExtGithub(input: $input) {
    updatedCount
    skippedCount
    totalCount
  }
}
    `;
export const CreateOrganizationDocument = gql`
    mutation CreateOrganization($input: CreateOrganizationInput!) {
  createOrganization(input: $input) {
    id
    name
    username
    description
  }
}
    `;

export type SdkFunctionWrapper = <T>(action: (requestHeaders?:Record<string, string>) => Promise<T>, operationName: string, operationType?: string) => Promise<T>;


const defaultWrapper: SdkFunctionWrapper = (action, _operationName, _operationType) => action();

export function getSdk(client: GraphQLClient, withWrapper: SdkFunctionWrapper = defaultWrapper) {
  return {
    verify(variables: VerifyMutationVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<VerifyMutation> {
      return withWrapper((wrappedRequestHeaders) => client.request<VerifyMutation>(VerifyDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'verify', 'mutation');
    },
    signInOrSignUp(variables: SignInOrSignUpMutationVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<SignInOrSignUpMutation> {
      return withWrapper((wrappedRequestHeaders) => client.request<SignInOrSignUpMutation>(SignInOrSignUpDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'signInOrSignUp', 'mutation');
    },
    me(variables?: MeQueryVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<MeQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<MeQuery>(MeDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'me', 'query');
    },
    createOperator(variables: CreateOperatorMutationVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<CreateOperatorMutation> {
      return withWrapper((wrappedRequestHeaders) => client.request<CreateOperatorMutation>(CreateOperatorDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'createOperator', 'mutation');
    },
    dashboard(variables?: DashboardQueryVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<DashboardQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<DashboardQuery>(DashboardDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'dashboard', 'query');
    },
    dashboardOrgRepos(variables: DashboardOrgReposQueryVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<DashboardOrgReposQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<DashboardOrgReposQuery>(DashboardOrgReposDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'dashboardOrgRepos', 'query');
    },
    ErrTest(variables?: ErrTestQueryVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<ErrTestQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<ErrTestQuery>(ErrTestDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'ErrTest', 'query');
    },
    newRepoPage(variables?: NewRepoPageQueryVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<NewRepoPageQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<NewRepoPageQuery>(NewRepoPageDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'newRepoPage', 'query');
    },
    dataDetailPage(variables: DataDetailPageQueryVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<DataDetailPageQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<DataDetailPageQuery>(DataDetailPageDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'dataDetailPage', 'query');
    },
    updateData(variables: UpdateDataMutationVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<UpdateDataMutation> {
      return withWrapper((wrappedRequestHeaders) => client.request<UpdateDataMutation>(UpdateDataDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'updateData', 'mutation');
    },
    dataOgpMeta(variables: DataOgpMetaQueryVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<DataOgpMetaQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<DataOgpMetaQuery>(DataOgpMetaDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'dataOgpMeta', 'query');
    },
    newData(variables: NewDataQueryVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<NewDataQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<NewDataQuery>(NewDataDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'newData', 'query');
    },
    addData(variables: AddDataMutationVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<AddDataMutation> {
      return withWrapper((wrappedRequestHeaders) => client.request<AddDataMutation>(AddDataDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'addData', 'mutation');
    },
    repoOgpMeta(variables: RepoOgpMetaQueryVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<RepoOgpMetaQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<RepoOgpMetaQuery>(RepoOgpMetaDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'repoOgpMeta', 'query');
    },
    properties(variables: PropertiesQueryVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<PropertiesQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<PropertiesQuery>(PropertiesDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'properties', 'query');
    },
    addProperty(variables: AddPropertyMutationVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<AddPropertyMutation> {
      return withWrapper((wrappedRequestHeaders) => client.request<AddPropertyMutation>(AddPropertyDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'addProperty', 'mutation');
    },
    updateProperty(variables: UpdatePropertyMutationVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<UpdatePropertyMutation> {
      return withWrapper((wrappedRequestHeaders) => client.request<UpdatePropertyMutation>(UpdatePropertyDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'updateProperty', 'mutation');
    },
    deleteProperty(variables: DeletePropertyMutationVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<DeletePropertyMutation> {
      return withWrapper((wrappedRequestHeaders) => client.request<DeletePropertyMutation>(DeletePropertyDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'deleteProperty', 'mutation');
    },
    repositoryPageWithTags(variables: RepositoryPageWithTagsQueryVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<RepositoryPageWithTagsQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<RepositoryPageWithTagsQuery>(RepositoryPageWithTagsDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'repositoryPageWithTags', 'query');
    },
    repositoryPage(variables: RepositoryPageQueryVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<RepositoryPageQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<RepositoryPageQuery>(RepositoryPageDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'repositoryPage', 'query');
    },
    InviteRepoMember(variables: InviteRepoMemberMutationVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<InviteRepoMemberMutation> {
      return withWrapper((wrappedRequestHeaders) => client.request<InviteRepoMemberMutation>(InviteRepoMemberDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'InviteRepoMember', 'mutation');
    },
    RemoveRepoMember(variables: RemoveRepoMemberMutationVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<RemoveRepoMemberMutation> {
      return withWrapper((wrappedRequestHeaders) => client.request<RemoveRepoMemberMutation>(RemoveRepoMemberDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'RemoveRepoMember', 'mutation');
    },
    ChangeRepoMemberRole(variables: ChangeRepoMemberRoleMutationVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<ChangeRepoMemberRoleMutation> {
      return withWrapper((wrappedRequestHeaders) => client.request<ChangeRepoMemberRoleMutation>(ChangeRepoMemberRoleDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'ChangeRepoMemberRole', 'mutation');
    },
    GetRepoMembers(variables: GetRepoMembersQueryVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<GetRepoMembersQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<GetRepoMembersQuery>(GetRepoMembersDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'GetRepoMembers', 'query');
    },
    getRepoSettings(variables: GetRepoSettingsQueryVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<GetRepoSettingsQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<GetRepoSettingsQuery>(GetRepoSettingsDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getRepoSettings', 'query');
    },
    GetOrgSettings(variables: GetOrgSettingsQueryVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<GetOrgSettingsQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<GetOrgSettingsQuery>(GetOrgSettingsDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'GetOrgSettings', 'query');
    },
    UpdateRepoSettings(variables: UpdateRepoSettingsMutationVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<UpdateRepoSettingsMutation> {
      return withWrapper((wrappedRequestHeaders) => client.request<UpdateRepoSettingsMutation>(UpdateRepoSettingsDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'UpdateRepoSettings', 'mutation');
    },
    DeleteRepo(variables: DeleteRepoMutationVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<DeleteRepoMutation> {
      return withWrapper((wrappedRequestHeaders) => client.request<DeleteRepoMutation>(DeleteRepoDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'DeleteRepo', 'mutation');
    },
    ChangeRepoUsername(variables: ChangeRepoUsernameMutationVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<ChangeRepoUsernameMutation> {
      return withWrapper((wrappedRequestHeaders) => client.request<ChangeRepoUsernameMutation>(ChangeRepoUsernameDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'ChangeRepoUsername', 'mutation');
    },
    getRepoSettingsPage(variables: GetRepoSettingsPageQueryVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<GetRepoSettingsPageQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<GetRepoSettingsPageQuery>(GetRepoSettingsPageDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getRepoSettingsPage', 'query');
    },
    EnableGitHubSync(variables: EnableGitHubSyncMutationVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<EnableGitHubSyncMutation> {
      return withWrapper((wrappedRequestHeaders) => client.request<EnableGitHubSyncMutation>(EnableGitHubSyncDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'EnableGitHubSync', 'mutation');
    },
    DisableGitHubSync(variables: DisableGitHubSyncMutationVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<DisableGitHubSyncMutation> {
      return withWrapper((wrappedRequestHeaders) => client.request<DisableGitHubSyncMutation>(DisableGitHubSyncDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'DisableGitHubSync', 'mutation');
    },
    EnableLinearSync(variables: EnableLinearSyncMutationVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<EnableLinearSyncMutation> {
      return withWrapper((wrappedRequestHeaders) => client.request<EnableLinearSyncMutation>(EnableLinearSyncDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'EnableLinearSync', 'mutation');
    },
    createAPIKey(variables: CreateApiKeyMutationVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<CreateApiKeyMutation> {
      return withWrapper((wrappedRequestHeaders) => client.request<CreateApiKeyMutation>(CreateApiKeyDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'createAPIKey', 'mutation');
    },
    getApiKeys(variables: GetApiKeysQueryVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<GetApiKeysQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<GetApiKeysQuery>(GetApiKeysDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getApiKeys', 'query');
    },
    GitHubListDirectoryContents(variables: GitHubListDirectoryContentsQueryVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<GitHubListDirectoryContentsQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<GitHubListDirectoryContentsQuery>(GitHubListDirectoryContentsDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'GitHubListDirectoryContents', 'query');
    },
    GitHubGetMarkdownPreviews(variables: GitHubGetMarkdownPreviewsQueryVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<GitHubGetMarkdownPreviewsQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<GitHubGetMarkdownPreviewsQuery>(GitHubGetMarkdownPreviewsDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'GitHubGetMarkdownPreviews', 'query');
    },
    GitHubAnalyzeFrontmatter(variables: GitHubAnalyzeFrontmatterQueryVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<GitHubAnalyzeFrontmatterQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<GitHubAnalyzeFrontmatterQuery>(GitHubAnalyzeFrontmatterDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'GitHubAnalyzeFrontmatter', 'query');
    },
    ImportMarkdownFromGitHub(variables: ImportMarkdownFromGitHubMutationVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<ImportMarkdownFromGitHubMutation> {
      return withWrapper((wrappedRequestHeaders) => client.request<ImportMarkdownFromGitHubMutation>(ImportMarkdownFromGitHubDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'ImportMarkdownFromGitHub', 'mutation');
    },
    newDatabase(variables?: NewDatabaseQueryVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<NewDatabaseQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<NewDatabaseQuery>(NewDatabaseDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'newDatabase', 'query');
    },
    createRepoOnOrgNewDatabasePage(variables: CreateRepoOnOrgNewDatabasePageMutationVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<CreateRepoOnOrgNewDatabasePageMutation> {
      return withWrapper((wrappedRequestHeaders) => client.request<CreateRepoOnOrgNewDatabasePageMutation>(CreateRepoOnOrgNewDatabasePageDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'createRepoOnOrgNewDatabasePage', 'mutation');
    },
    orgOgpMeta(variables: OrgOgpMetaQueryVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<OrgOgpMetaQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<OrgOgpMetaQuery>(OrgOgpMetaDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'orgOgpMeta', 'query');
    },
    orgPage(variables: OrgPageQueryVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<OrgPageQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<OrgPageQuery>(OrgPageDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'orgPage', 'query');
    },
    updateOrgOnForm(variables: UpdateOrgOnFormMutationVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<UpdateOrgOnFormMutation> {
      return withWrapper((wrappedRequestHeaders) => client.request<UpdateOrgOnFormMutation>(UpdateOrgOnFormDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'updateOrgOnForm', 'mutation');
    },
    orgInvitePage(variables: OrgInvitePageQueryVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<OrgInvitePageQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<OrgInvitePageQuery>(OrgInvitePageDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'orgInvitePage', 'query');
    },
    inviteUser(variables: InviteUserMutationVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<InviteUserMutation> {
      return withWrapper((wrappedRequestHeaders) => client.request<InviteUserMutation>(InviteUserDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'inviteUser', 'mutation');
    },
    GitHubAuthUrl(variables: GitHubAuthUrlMutationVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<GitHubAuthUrlMutation> {
      return withWrapper((wrappedRequestHeaders) => client.request<GitHubAuthUrlMutation>(GitHubAuthUrlDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'GitHubAuthUrl', 'mutation');
    },
    GitHubExchangeToken(variables: GitHubExchangeTokenMutationVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<GitHubExchangeTokenMutation> {
      return withWrapper((wrappedRequestHeaders) => client.request<GitHubExchangeTokenMutation>(GitHubExchangeTokenDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'GitHubExchangeToken', 'mutation');
    },
    GitHubDisconnect(variables?: GitHubDisconnectMutationVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<GitHubDisconnectMutation> {
      return withWrapper((wrappedRequestHeaders) => client.request<GitHubDisconnectMutation>(GitHubDisconnectDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'GitHubDisconnect', 'mutation');
    },
    GitHubConnection(variables?: GitHubConnectionQueryVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<GitHubConnectionQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<GitHubConnectionQuery>(GitHubConnectionDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'GitHubConnection', 'query');
    },
    GitHubListRepositories(variables?: GitHubListRepositoriesQueryVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<GitHubListRepositoriesQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<GitHubListRepositoriesQuery>(GitHubListRepositoriesDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'GitHubListRepositories', 'query');
    },
    SyncDataToGithub(variables: SyncDataToGithubMutationVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<SyncDataToGithubMutation> {
      return withWrapper((wrappedRequestHeaders) => client.request<SyncDataToGithubMutation>(SyncDataToGithubDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'SyncDataToGithub', 'mutation');
    },
    DataCountForBulkSync(variables: DataCountForBulkSyncQueryVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<DataCountForBulkSyncQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<DataCountForBulkSyncQuery>(DataCountForBulkSyncDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'DataCountForBulkSync', 'query');
    },
    BulkSyncExtGithub(variables: BulkSyncExtGithubMutationVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<BulkSyncExtGithubMutation> {
      return withWrapper((wrappedRequestHeaders) => client.request<BulkSyncExtGithubMutation>(BulkSyncExtGithubDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'BulkSyncExtGithub', 'mutation');
    },
    CreateOrganization(variables: CreateOrganizationMutationVariables, requestHeaders?: GraphQLClientRequestHeaders): Promise<CreateOrganizationMutation> {
      return withWrapper((wrappedRequestHeaders) => client.request<CreateOrganizationMutation>(CreateOrganizationDocument, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'CreateOrganization', 'mutation');
    }
  };
}
export type Sdk = ReturnType<typeof getSdk>;

type Properties<T> = Required<{
  [K in keyof T]: z.ZodType<T[K], any, T[K]>;
}>;

type definedNonNullAny = {};

export const isDefinedNonNullAny = (v: any): v is definedNonNullAny => v !== undefined && v !== null;

export const definedNonNullAnySchema = z.any().refine((v) => isDefinedNonNullAny(v));

export const DefaultRoleSchema = z.nativeEnum(DefaultRole);

export const GqlConnectionActionSchema = z.nativeEnum(GqlConnectionAction);

export const GqlConnectionStatusSchema = z.nativeEnum(GqlConnectionStatus);

export const GqlEndpointStatusSchema = z.nativeEnum(GqlEndpointStatus);

export const GqlIntegrationCategorySchema = z.nativeEnum(GqlIntegrationCategory);

export const GqlProcessingStatusSchema = z.nativeEnum(GqlProcessingStatus);

export const GqlProviderSchema = z.nativeEnum(GqlProvider);

export const GqlSyncCapabilitySchema = z.nativeEnum(GqlSyncCapability);

export const GqlSyncOperationStatusSchema = z.nativeEnum(GqlSyncOperationStatus);

export const GqlSyncOperationTypeSchema = z.nativeEnum(GqlSyncOperationType);

export const NewOperatorOwnerMethodSchema = z.nativeEnum(NewOperatorOwnerMethod);

export const OrgRoleSchema = z.nativeEnum(OrgRole);

export const PermissionSourceSchema = z.nativeEnum(PermissionSource);

export const PropertyTypeSchema = z.nativeEnum(PropertyType);

export const SyncStatusSchema = z.nativeEnum(SyncStatus);

export function AddDataInputDataSchema(): z.ZodObject<Properties<AddDataInputData>> {
  return z.object({
    actor: z.string().min(1),
    dataName: z.string().min(1),
    orgUsername: z.string().min(1),
    propertyData: z.array(z.lazy(() => PropertyDataInputDataSchema())),
    repoUsername: z.string().min(1)
  })
}

export function BulkSyncExtGithubInputSchema(): z.ZodObject<Properties<BulkSyncExtGithubInput>> {
  return z.object({
    extGithubPropertyId: z.string().min(1),
    orgUsername: z.string().min(1),
    repoConfigs: z.array(z.lazy(() => ExtGithubRepoConfigInputSchema())),
    repoUsername: z.string().min(1)
  })
}

export function ChangeOrgMemberRoleInputSchema(): z.ZodObject<Properties<ChangeOrgMemberRoleInput>> {
  return z.object({
    newRole: OrgRoleSchema,
    tenantId: z.string().min(1),
    userId: z.string().min(1)
  })
}

export function ChangeRepoMemberRoleInputSchema(): z.ZodObject<Properties<ChangeRepoMemberRoleInput>> {
  return z.object({
    newRole: z.string().min(1),
    repoId: z.string().min(1),
    userId: z.string().min(1)
  })
}

export function ChangeRepoUsernameInputSchema(): z.ZodObject<Properties<ChangeRepoUsernameInput>> {
  return z.object({
    newRepoUsername: z.string().min(1),
    oldRepoUsername: z.string().min(1),
    orgUsername: z.string().min(1)
  })
}

export function ConnectIntegrationInputSchema(): z.ZodObject<Properties<ConnectIntegrationInput>> {
  return z.object({
    apiKey: z.string().nullish(),
    authCode: z.string().nullish(),
    integrationId: z.string().min(1)
  })
}

export function CreateApiKeyInputSchema(): z.ZodObject<Properties<CreateApiKeyInput>> {
  return z.object({
    name: z.string().min(1),
    organizationUsername: z.string().min(1),
    serviceAccountName: z.string().nullish()
  })
}

export function CreateOperatorInputSchema(): z.ZodObject<Properties<CreateOperatorInput>> {
  return z.object({
    newOperatorOwnerId: z.string().min(1),
    newOperatorOwnerMethod: NewOperatorOwnerMethodSchema,
    newOperatorOwnerPassword: z.string().nullish(),
    operatorAlias: z.string().nullish(),
    operatorName: z.string().min(1),
    platformId: z.string().min(1)
  })
}

export function CreateOrganizationInputSchema(): z.ZodObject<Properties<CreateOrganizationInput>> {
  return z.object({
    description: z.string().nullish(),
    name: z.string().min(1),
    username: z.string().min(1),
    website: z.string().nullish()
  })
}

export function CreateRepoInputSchema(): z.ZodObject<Properties<CreateRepoInput>> {
  return z.object({
    databaseId: z.string().nullish(),
    description: z.string().nullish(),
    isPublic: z.boolean(),
    orgUsername: z.string().min(1),
    repoName: z.string().min(1),
    repoUsername: z.string().min(1),
    userId: z.string().min(1)
  })
}

export function CreateSourceInputSchema(): z.ZodObject<Properties<CreateSourceInput>> {
  return z.object({
    name: z.string().min(1),
    orgUsername: z.string().min(1),
    repoUsername: z.string().min(1),
    url: z.string().nullish()
  })
}

export function CreateWebhookEndpointInputSchema(): z.ZodObject<Properties<CreateWebhookEndpointInput>> {
  return z.object({
    config: z.string().min(1),
    events: z.array(z.string().min(1)),
    mapping: z.string().nullish(),
    name: z.string().min(1),
    provider: GqlProviderSchema,
    repositoryId: z.string().nullish()
  })
}

export function DisableGitHubSyncInputSchema(): z.ZodObject<Properties<DisableGitHubSyncInput>> {
  return z.object({
    orgUsername: z.string().min(1),
    repoUsername: z.string().min(1)
  })
}

export function EnableGitHubSyncInputSchema(): z.ZodObject<Properties<EnableGitHubSyncInput>> {
  return z.object({
    orgUsername: z.string().min(1),
    repoUsername: z.string().min(1)
  })
}

export function EnableLinearSyncInputSchema(): z.ZodObject<Properties<EnableLinearSyncInput>> {
  return z.object({
    orgUsername: z.string().min(1),
    repoUsername: z.string().min(1)
  })
}

export function ExchangeOAuthCodeInputSchema(): z.ZodObject<Properties<ExchangeOAuthCodeInput>> {
  return z.object({
    code: z.string().min(1),
    integrationId: z.string().min(1),
    redirectUri: z.string().min(1),
    state: z.string().nullish()
  })
}

export function ExtGithubRepoConfigInputSchema(): z.ZodObject<Properties<ExtGithubRepoConfigInput>> {
  return z.object({
    defaultPath: z.string().nullish(),
    label: z.string().nullish(),
    repo: z.string().min(1)
  })
}

export function GetMarkdownPreviewsInputSchema(): z.ZodObject<Properties<GetMarkdownPreviewsInput>> {
  return z.object({
    githubRepo: z.string().min(1),
    paths: z.array(z.string().min(1)),
    refName: z.string().nullish()
  })
}

export function IdOrEmailSchema(): z.ZodObject<Properties<IdOrEmail>> {
  return z.object({
    email: z.string().nullish(),
    id: z.string().nullish()
  })
}

export function ImportMarkdownFromGitHubInputSchema(): z.ZodObject<Properties<ImportMarkdownFromGitHubInput>> {
  return z.object({
    contentPropertyName: z.string().min(1),
    enableGithubSync: z.boolean().nullish(),
    githubRepo: z.string().min(1),
    orgUsername: z.string().min(1),
    paths: z.array(z.string().min(1)),
    propertyMappings: z.array(z.lazy(() => PropertyMappingInputSchema())),
    refName: z.string().nullish(),
    repoName: z.string().nullish(),
    repoUsername: z.string().min(1),
    skipExisting: z.boolean().nullish()
  })
}

export function InitOAuthInputSchema(): z.ZodObject<Properties<InitOAuthInput>> {
  return z.object({
    integrationId: z.string().min(1),
    redirectUri: z.string().nullish(),
    state: z.string().nullish()
  })
}

export function InviteRepoMemberInputSchema(): z.ZodObject<Properties<InviteRepoMemberInput>> {
  return z.object({
    orgUsername: z.string().min(1),
    repoId: z.string().min(1),
    repoUsername: z.string().min(1),
    role: z.string().min(1),
    usernameOrEmail: z.string().min(1)
  })
}

export function ListGitHubDirectoryInputSchema(): z.ZodObject<Properties<ListGitHubDirectoryInput>> {
  return z.object({
    githubRepo: z.string().min(1),
    path: z.string().min(1),
    recursive: z.boolean().nullish(),
    refName: z.string().nullish()
  })
}

export function LocationSchema(): z.ZodObject<Properties<Location>> {
  return z.object({
    latitude: z.number(),
    longitude: z.number()
  })
}

export function OptionInputSchema(): z.ZodObject<Properties<OptionInput>> {
  return z.object({
    identifier: z.string().min(1),
    label: z.string().min(1)
  })
}

export function PropertyDataInputDataSchema(): z.ZodObject<Properties<PropertyDataInputData>> {
  return z.object({
    propertyId: z.string().min(1),
    value: z.lazy(() => PropertyDataValueInputDataSchema())
  })
}

export function PropertyDataValueInputDataSchema(): z.ZodObject<Properties<PropertyDataValueInputData>> {
  return z.object({
    date: z.string().nullish(),
    html: z.string().nullish(),
    image: z.string().nullish(),
    integer: z.string().nullish(),
    location: LocationSchema().nullish(),
    markdown: z.string().nullish(),
    multiSelect: z.array(z.string().min(1)).nullish(),
    relation: z.array(z.string().min(1)).nullish(),
    select: z.string().nullish(),
    string: z.string().nullish()
  })
}

export function PropertyInputSchema(): z.ZodObject<Properties<PropertyInput>> {
  return z.object({
    meta: z.lazy(() => PropertyMetaInputSchema().nullish()),
    orgUsername: z.string().min(1),
    propertyName: z.string().min(1),
    propertyType: PropertyTypeSchema,
    repoUsername: z.string().min(1)
  })
}

export function PropertyMappingInputSchema(): z.ZodObject<Properties<PropertyMappingInput>> {
  return z.object({
    frontmatterKey: z.string().min(1),
    propertyName: z.string().min(1),
    propertyType: PropertyTypeSchema,
    selectOptions: z.array(z.string().min(1)).nullish()
  })
}

export function PropertyMetaInputSchema(): z.ZodObject<Properties<PropertyMetaInput>> {
  return z.object({
    id: z.boolean().nullish(),
    json: z.string().nullish(),
    multiSelect: z.array(z.lazy(() => OptionInputSchema())).nullish(),
    relation: z.string().nullish(),
    select: z.array(z.lazy(() => OptionInputSchema())).nullish()
  })
}

export function RemoveRepoMemberInputSchema(): z.ZodObject<Properties<RemoveRepoMemberInput>> {
  return z.object({
    repoId: z.string().min(1),
    userId: z.string().min(1)
  })
}

export function StartInitialSyncInputSchema(): z.ZodObject<Properties<StartInitialSyncInput>> {
  return z.object({
    endpointId: z.string().min(1)
  })
}

export function SyncToGitHubInputSchema(): z.ZodObject<Properties<SyncToGitHubInput>> {
  return z.object({
    commitMessage: z.string().nullish(),
    dataId: z.string().min(1),
    dryRun: z.boolean().nullish(),
    orgUsername: z.string().min(1),
    repoUsername: z.string().min(1),
    targetBranch: z.string().nullish(),
    targetPath: z.string().min(1),
    targetRepo: z.string().min(1)
  })
}

export function TriggerSyncInputSchema(): z.ZodObject<Properties<TriggerSyncInput>> {
  return z.object({
    endpointId: z.string().min(1),
    externalIds: z.array(z.string().min(1)).nullish()
  })
}

export function UpdateDataInputDataSchema(): z.ZodObject<Properties<UpdateDataInputData>> {
  return z.object({
    actor: z.string().min(1),
    dataId: z.string().min(1),
    dataName: z.string().min(1),
    orgUsername: z.string().min(1),
    propertyData: z.array(z.lazy(() => PropertyDataInputDataSchema())),
    repoUsername: z.string().min(1)
  })
}

export function UpdateEndpointConfigInputSchema(): z.ZodObject<Properties<UpdateEndpointConfigInput>> {
  return z.object({
    config: z.string().min(1),
    endpointId: z.string().min(1)
  })
}

export function UpdateEndpointEventsInputSchema(): z.ZodObject<Properties<UpdateEndpointEventsInput>> {
  return z.object({
    endpointId: z.string().min(1),
    events: z.array(z.string().min(1))
  })
}

export function UpdateEndpointMappingInputSchema(): z.ZodObject<Properties<UpdateEndpointMappingInput>> {
  return z.object({
    endpointId: z.string().min(1),
    mapping: z.string().nullish()
  })
}

export function UpdateEndpointStatusInputSchema(): z.ZodObject<Properties<UpdateEndpointStatusInput>> {
  return z.object({
    endpointId: z.string().min(1),
    status: GqlEndpointStatusSchema
  })
}

export function UpdateOrganizationInputSchema(): z.ZodObject<Properties<UpdateOrganizationInput>> {
  return z.object({
    description: z.string().nullish(),
    name: z.string().min(1),
    username: z.string().min(1),
    website: z.string().nullish()
  })
}

export function UpdateRepoInputSchema(): z.ZodObject<Properties<UpdateRepoInput>> {
  return z.object({
    description: z.string().nullish(),
    isPublic: z.boolean().nullish(),
    name: z.string().nullish(),
    orgUsername: z.string().min(1),
    repoUsername: z.string().min(1),
    tags: z.array(z.string().min(1)).nullish()
  })
}

export function UpdateSourceInputSchema(): z.ZodObject<Properties<UpdateSourceInput>> {
  return z.object({
    name: z.string().nullish(),
    orgUsername: z.string().min(1),
    repoUsername: z.string().min(1),
    sourceId: z.string().min(1),
    url: z.string().nullish()
  })
}
