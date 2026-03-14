/* eslint-disable */
import type { DefineMethods } from 'aspida';
import type * as Types from '../@types';

export type Methods = DefineMethods<{
  /** Sign up */
  post: {
    status: 201;
    /** Created */
    resBody: Types.SignUpResponse;
    reqBody: Types.SignUpRequest;
  };
}>;
