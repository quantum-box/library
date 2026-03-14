/* eslint-disable */
export type AddDataRequest = {
  name: string;
  property_data: PropertyDataRequest[];
}

export type AddPropertyRequest = {
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
}

export type PropertyResponse = {
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
