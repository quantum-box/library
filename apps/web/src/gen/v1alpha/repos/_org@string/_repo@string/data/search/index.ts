/* eslint-disable */
import type { DefineMethods } from 'aspida';
import type * as Types from '../../../../../@types';

export type Methods = DefineMethods<{
  /** Search data by name */
  get: {
    query: {
      search: string;
    };

    status: 200;
    /** data */
    resBody: Types.WithPaginator_for_Array_of_Data;
  };
}>;
