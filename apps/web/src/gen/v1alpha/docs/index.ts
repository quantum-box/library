/* eslint-disable */
import type { DefineMethods } from 'aspida';

export type Methods = DefineMethods<{
  /** This documentation page. */
  get: {
    status: 200;
    /** HTML content */
    resBody: string;
  };
}>;
