/* eslint-disable */
export type AddDataRequest = {
  name: string;
  property_data: PropertyDataRequest[];
}

export type AddPropertyRequest = {
  /** Versioned numeric or computed Property configuration. */
  config?: Record<string, unknown> | null;
  auto_generate?: boolean | null;
  name: string;
  property_type: string;
}

export type AuthUrlResponse = {
  /** OAuth authorization URL */
  url: string;
}

export type ChangeRepoUsernameRequest = {
  new_username: string;
}

export type CreateOrganizationRequest = {
  name: string;
  username: string;
}

export type CreateRepoRequest = {
  is_public: boolean;
  name: string;
  username: string;
}

export type CreateSourceRequest = {
  name: string;
}

export type CreateUserRequest = {
  email: string;
  operator_id: string;
  username: string;
}

export type CreateUserResponse = {
  user: User;
}

export type DataResponse = {
  id: string;
  items: PropertyDataResponse[];
  name: string;
  /** Decimal Library record revision. */
  recordVersion: string;
}

/**
 * # Location
 * 位置情報（緯度・経度）を表すValueObject
 */
export type Location = {
  /** 緯度 (-90.0 ~ 90.0) */
  latitude: number;
  /** 経度 (-180.0 ~ 180.0) */
  longitude: number;
}

export type OAuthCallbackResponse = {
  operatorId: string;
}

export type OrganizationResponse = {
  id: string;
  name: string;
  repos: RepoResponse[];
  username: string;
}

export type PropertyDataRequest = {
  property_id: string;
}

export type PropertyDataResponse = {
  key: string;
  property_id: string;

  value?: null | PropertyDataValue | undefined;
}

export type PropertyDataValue = {
  string: string;
} | {
  integer: number;
} | {
  html: string;
} | {
  markdown: string;
} | {
  relation: {
    data_id: string[];
    database_id: string;
  };
} | {
  id: string;
} | {
  select: string;
} | {
  multiSelect: string[];
} | {
  location: Location;
} | {
  /** YYYY-MM-DD or RFC 3339 date-time; responses use canonical UTC. */
  date: string;
} | {
  image: string;
} | {
  /** A block document, carried as JSON text. */
  richText: string;
} | {
  boolean: boolean;
} | {
  decimal: string;
} | {
  quantity: QuantityValue;
} | {
  formula: ComputedResult;
} | {
  rollup: ComputedResult;
} | {
  richTextPreview: TextPreviewResponse;
}

export type PropertyResponse = {
  /** Versioned numeric or computed Property configuration. */
  config?: Record<string, unknown> | null;
  auto_generate?: boolean | null;
  database_id?: string | null;
  id: string;
  name: string;
  property_type: string;
}

export type RepoResponse = {
  id: string;
  is_public: boolean;
  name: string;
  organization_id: string;
  username: string;
}

export type SearchDataQuery = {
  name: string;
}

export type SearchRepoQuery = {
}

export type SourceResponse = {
  id: string;
  name: string;
  repo_id: string;
}

export type UpdateDataRequest = {
  name: string;
  property_data: PropertyDataRequest[];
}

export type UpdateOrganizationRequest = {
  name: string;
}

export type UpdatePropertyRequest = {
  property_type?: string | null;
  /** Versioned numeric or computed Property configuration. */
  config?: Record<string, unknown> | null;
  name: string;
}

export type UpdateRepoRequest = {
}

export type UpdateSourceRequest = {
}

export type User = {
  id: string;
  role: string;
}

export type VerifyRequest = {
  token: string;
}

export type VerifyResponse = {
  user: User;
}

export type ComputedResult = {
  status: 'ready';
  value: string;
} | {
  status: 'null';
} | {
  status: 'pending';
} | {
  code: string;
  status: 'error';
}

export type QuantityValue = {
  unit: string;
  value: string;
}

export type TextPreviewResponse = {
  /** The leading text, at most `RICH_TEXT_PREVIEW_LIMIT` characters. */
  text: string;
  /** Whether the body holds more text than `text` shows. */
  truncated: boolean;
}
