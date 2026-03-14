/* eslint-disable */
import type { DefineMethods } from 'aspida';
import type * as Types from '../../../../../@types';

export type Methods = DefineMethods<{
  get: {
    status: 200;
    /** Properties found */
    resBody: Types.PropertyResponse[];
  };

  post: {
    status: 201;
    /** Property created */
    resBody: Types.PropertyResponse;
    reqBody: Types.AddPropertyRequest;
  };
}>;
