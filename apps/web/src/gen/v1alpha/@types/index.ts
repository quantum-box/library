/* eslint-disable */
export type AddPropertyInput = {
  property_name: string;
  property_type: string;
}

export type AddPropertyInputPath = {
  org: string;
  repo: string;
}

/** A default error response for most API errors. */
export type AppError = {
  /** An error message. */
  error: string;
  /** A unique error ID. */
  error_id: string;
}

export type CreateDataPath = {
  org: string;
  repo: string;
}

export type CreateDataRequest = {
  name: string;
  property_data: PropertyDataRequest[];
}

export type Data = {
  id: string;
  name: string;
  property_data: PropertyData[];
}

export type DeleteDataPath = {
  data_id: string;
  org: string;
  repo: string;
}

export type DeletePropertyInputPath = {
  org: string;
  property_id: string;
  repo: string;
}

export type DeleteRepoPath = {
  org: string;
  repo: string;
}

export type GetDataListPath = {
  org: string;
  repo: string;
}

export type GetDataListQuery = {
}

export type GetDataPath = {
  data_id: string;
  org: string;
  repo: string;
}

export type GetPropertiesInputPath = {
  org: string;
  repo: string;
}

export type GetRepoPath = {
  org: string;
  repo: string;
}

export type Paginator = {
  current_page: number;
  items_per_page: number;
  total_items: number;
  total_pages: number;
}

export type Property = {
  id: string;
  name: string;
  property_type: string;
}

export type PropertyData = {
  property_id: string;
  property_type: string;
  value: string;
}

export type PropertyDataRequest = {
  property_id: string;
  value: string;
}

export type Repo = {
  id: string;
  name: string;
}

export type SearchDataPath = {
  org: string;
  repo: string;
}

export type SearchDataQuery = {
  search: string;
}

export type SearchInOrgPath = {
  org: string;
}

export type SearchInOrgQuery = {
}

export type SearchQuery = {
}

export type SignUpRequest = {
}

export type SignUpResponse = {
  user_id: string;
}

export type UpdateDataInput = {
  data: UpdatePropertyDataInput[];
  name: string;
}

export type UpdateDataPath = {
  data_id: string;
  org: string;
  repo: string;
}

export type UpdatePropertyDataInput = {
  data: string;
  property_id: string;
}

export type WithPaginator_for_Array_of_Data = {
  items: Data[];
  paginator: Paginator;
}
