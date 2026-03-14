/* eslint-disable */
import type { DefineMethods } from 'aspida';
import type * as Types from '../../../../../../@types';

export type Methods = DefineMethods<{
  get: {
    status: 200;
    /** Source found */
    resBody: Types.SourceResponse;
  };

  put: {
    status: 200;
    /** Source updated */
    resBody: Types.SourceResponse;
    reqBody: Types.UpdateSourceRequest;
  };

  delete: {
    status: 204;
  };
}>;
