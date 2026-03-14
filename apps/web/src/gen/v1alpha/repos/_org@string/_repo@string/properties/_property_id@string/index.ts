/* eslint-disable */
import type { DefineMethods } from 'aspida';
import type * as Types from '../../../../../@types';

export type Methods = DefineMethods<{
  /** Delete property */
  delete: {
    status: 200;
    /** Deleted */
    resBody: Types.Property;
  };
}>;
