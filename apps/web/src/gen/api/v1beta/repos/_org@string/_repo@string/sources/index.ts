/* eslint-disable */
import type { DefineMethods } from 'aspida';
import type * as Types from '../../../../../@types';

export type Methods = DefineMethods<{
  get: {
    status: 200;
    /** Sources found */
    resBody: Types.SourceResponse[];
  };

  post: {
    status: 201;
    /** Source created */
    resBody: Types.SourceResponse;
    reqBody: Types.CreateSourceRequest;
  };
}>;
