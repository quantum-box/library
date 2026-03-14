/* eslint-disable */
import type { DefineMethods } from 'aspida';
import type * as Types from '../../../@types';

export type Methods = DefineMethods<{
  /** Get a repo by id. */
  get: {
    status: 200;
    /** A repo. */
    resBody: Types.Repo;
  };

  /** Delete a repo by id. */
  delete: {
  };
}>;
