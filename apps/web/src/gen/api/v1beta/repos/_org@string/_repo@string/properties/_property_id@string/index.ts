/* eslint-disable */
import type { DefineMethods } from 'aspida';
import type * as Types from '../../../../../../@types';

export type Methods = DefineMethods<{
  get: {
    status: 200;
    /** Property found */
    resBody: Types.PropertyResponse;
  };

  put: {
    status: 200;
    /** Property updated */
    resBody: Types.PropertyResponse;
    reqBody: Types.UpdatePropertyRequest;
  };

  delete: {
    status: 204;
  };
}>;
