/* eslint-disable */
import type { DefineMethods } from 'aspida';
import type * as Types from '../../../../@types';

export type Methods = DefineMethods<{
  /** Get properties */
  get: {
    status: 200;
    /** OK */
    resBody: Types.Property[];
  };

  /** Add property */
  post: {
    status: 201;
    /** Created */
    resBody: Types.Property;
    reqBody: Types.AddPropertyInput;
  };
}>;
