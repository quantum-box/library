/* eslint-disable */
import type { DefineMethods } from 'aspida';
import type * as Types from '../../../../../@types';

export type Methods = DefineMethods<{
  /** get data by id */
  get: {
    status: 200;
    /** data */
    resBody: Types.Data;
  };

  /** Update data by id */
  put: {
    status: 201;
    /** update data */
    resBody: Types.Data;
    reqBody: Types.UpdateDataInput;
  };

  /** delete data by id */
  delete: {
    status: 204;
  };
}>;
