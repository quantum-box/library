/* eslint-disable */
import type { DefineMethods } from 'aspida';
import type * as Types from '../../../../@types';

export type Methods = DefineMethods<{
  get: {
    status: 200;
    /** Repository found */
    resBody: Types.RepoResponse;
  };

  put: {
    status: 200;
    /** Repository updated */
    resBody: Types.RepoResponse;
    reqBody: Types.UpdateRepoRequest;
  };

  delete: {
    status: 204;
  };
}>;
