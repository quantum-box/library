/* eslint-disable */
import type { DefineMethods } from 'aspida';
import type * as Types from '../../../../../../@types';

export type Methods = DefineMethods<{
  get: {
  };

  put: {
    status: 200;
    /** Data updated */
    resBody: Types.DataResponse;
    reqBody: Types.UpdateDataRequest;
  };

  delete: {
    status: 204;
  };
}>;
