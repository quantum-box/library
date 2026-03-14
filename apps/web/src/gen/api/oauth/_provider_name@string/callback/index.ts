/* eslint-disable */
import type { DefineMethods } from 'aspida';
import type * as Types from '../../../@types';

export type Methods = DefineMethods<{
  get: {
    query: {
      /** Authorization code from provider */
      code: string;
      /** State parameter for CSRF protection */
      state: string;
    };

    status: 200;
    /** Successfully connected to provider */
    resBody: Types.OAuthCallbackResponse;
  };
}>;
