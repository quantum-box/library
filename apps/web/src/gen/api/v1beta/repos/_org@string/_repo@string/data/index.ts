/* eslint-disable */
import type { DefineMethods } from 'aspida';
import type * as Types from '../../../../../@types';

export type Methods = DefineMethods<{
  get: {
    query: {
      /** Data name to search for */
      name: string;
    };

    status: 200;
    /** Data found */
    resBody: Types.DataResponse[];
  };

  post: {
    status: 201;
    /** Data created */
    resBody: Types.DataResponse;
    reqBody: Types.AddDataRequest;
  };
}>;
