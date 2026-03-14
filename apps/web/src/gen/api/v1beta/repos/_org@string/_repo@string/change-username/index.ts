/* eslint-disable */
import type { DefineMethods } from 'aspida';
import type * as Types from '../../../../../@types';

export type Methods = DefineMethods<{
  put: {
    status: 200;
    /** Repository username changed */
    resBody: Types.RepoResponse;
    reqBody: Types.ChangeRepoUsernameRequest;
  };
}>;
