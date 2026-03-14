/* eslint-disable */
import type { DefineMethods } from 'aspida';
import type * as Types from '../../../@types';

export type Methods = DefineMethods<{
  post: {
    status: 201;
    /** Repository created */
    resBody: Types.RepoResponse;
    reqBody: Types.CreateRepoRequest;
  };
}>;
