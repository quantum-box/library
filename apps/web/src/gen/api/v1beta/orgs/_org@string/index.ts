/* eslint-disable */
import type { DefineMethods } from 'aspida';
import type * as Types from '../../../@types';

export type Methods = DefineMethods<{
  get: {
    status: 200;
    /** Organization found */
    resBody: Types.OrganizationResponse;
  };

  put: {
    status: 200;
    /** Organization updated */
    resBody: Types.OrganizationResponse;
    reqBody: Types.UpdateOrganizationRequest;
  };
}>;
