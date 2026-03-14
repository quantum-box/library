/* eslint-disable */
import type { DefineMethods } from 'aspida';
import type * as Types from '../../../../@types';

export type Methods = DefineMethods<{
  /** get all data in repo */
  get: {
    status: 200;
    /** data */
    resBody: Types.WithPaginator_for_Array_of_Data;
  };

  /** create data */
  post: {
    status: 201;
    /** create data */
    resBody: Types.Data;
    reqBody: Types.CreateDataRequest;
  };
}>;
