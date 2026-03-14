import type { AspidaClient, BasicHeaders } from 'aspida';
import { dataToURLString } from 'aspida';
import type { Methods as Methods_k87v2t } from './docs';
import type { Methods as Methods_chr4nt } from './repos/_org@string/_repo@string';
import type { Methods as Methods_rkwoeq } from './repos/_org@string/_repo@string/data';
import type { Methods as Methods_c4c91d } from './repos/_org@string/_repo@string/data/_data_id@string';
import type { Methods as Methods_1wx4fwn } from './repos/_org@string/_repo@string/data/search';
import type { Methods as Methods_xq9h73 } from './repos/_org@string/_repo@string/properties';
import type { Methods as Methods_lpkfsx } from './repos/_org@string/_repo@string/properties/_property_id@string';
import type { Methods as Methods_1uz1w0l } from './sign_up';

const api = <T>({ baseURL, fetch }: AspidaClient<T>) => {
  const prefix = (baseURL === undefined ? '' : baseURL).replace(/\/$/, '');
  const PATH0 = '/docs';
  const PATH1 = '/repos';
  const PATH2 = '/data';
  const PATH3 = '/data/search';
  const PATH4 = '/properties';
  const PATH5 = '/sign_up';
  const GET = 'GET';
  const POST = 'POST';
  const PUT = 'PUT';
  const DELETE = 'DELETE';

  return {
    docs: {
      /**
       * This documentation page.
       * @returns HTML content
       */
      get: (option?: { config?: T | undefined } | undefined) =>
        fetch<Methods_k87v2t['get']['resBody'], BasicHeaders, Methods_k87v2t['get']['status']>(prefix, PATH0, GET, option).text(),
      /**
       * This documentation page.
       * @returns HTML content
       */
      $get: (option?: { config?: T | undefined } | undefined) =>
        fetch<Methods_k87v2t['get']['resBody'], BasicHeaders, Methods_k87v2t['get']['status']>(prefix, PATH0, GET, option).text().then(r => r.body),
      $path: () => `${prefix}${PATH0}`,
    },
    repos: {
      _org: (val1: string) => {
        const prefix1 = `${PATH1}/${val1}`;

        return {
          _repo: (val2: string) => {
            const prefix2 = `${prefix1}/${val2}`;

            return {
              data: {
                _data_id: (val4: string) => {
                  const prefix4 = `${prefix2}${PATH2}/${val4}`;

                  return {
                    /**
                     * get data by id
                     * @returns data
                     */
                    get: (option?: { config?: T | undefined } | undefined) =>
                      fetch<Methods_c4c91d['get']['resBody'], BasicHeaders, Methods_c4c91d['get']['status']>(prefix, prefix4, GET, option).json(),
                    /**
                     * get data by id
                     * @returns data
                     */
                    $get: (option?: { config?: T | undefined } | undefined) =>
                      fetch<Methods_c4c91d['get']['resBody'], BasicHeaders, Methods_c4c91d['get']['status']>(prefix, prefix4, GET, option).json().then(r => r.body),
                    /**
                     * Update data by id
                     * @returns update data
                     */
                    put: (option: { body: Methods_c4c91d['put']['reqBody'], config?: T | undefined }) =>
                      fetch<Methods_c4c91d['put']['resBody'], BasicHeaders, Methods_c4c91d['put']['status']>(prefix, prefix4, PUT, option).json(),
                    /**
                     * Update data by id
                     * @returns update data
                     */
                    $put: (option: { body: Methods_c4c91d['put']['reqBody'], config?: T | undefined }) =>
                      fetch<Methods_c4c91d['put']['resBody'], BasicHeaders, Methods_c4c91d['put']['status']>(prefix, prefix4, PUT, option).json().then(r => r.body),
                    /**
                     * delete data by id
                     */
                    delete: (option?: { config?: T | undefined } | undefined) =>
                      fetch<void, BasicHeaders, Methods_c4c91d['delete']['status']>(prefix, prefix4, DELETE, option).send(),
                    /**
                     * delete data by id
                     */
                    $delete: (option?: { config?: T | undefined } | undefined) =>
                      fetch<void, BasicHeaders, Methods_c4c91d['delete']['status']>(prefix, prefix4, DELETE, option).send().then(r => r.body),
                    $path: () => `${prefix}${prefix4}`,
                  };
                },
                search: {
                  /**
                   * Search data by name
                   * @returns data
                   */
                  get: (option: { query: Methods_1wx4fwn['get']['query'], config?: T | undefined }) =>
                    fetch<Methods_1wx4fwn['get']['resBody'], BasicHeaders, Methods_1wx4fwn['get']['status']>(prefix, `${prefix2}${PATH3}`, GET, option).json(),
                  /**
                   * Search data by name
                   * @returns data
                   */
                  $get: (option: { query: Methods_1wx4fwn['get']['query'], config?: T | undefined }) =>
                    fetch<Methods_1wx4fwn['get']['resBody'], BasicHeaders, Methods_1wx4fwn['get']['status']>(prefix, `${prefix2}${PATH3}`, GET, option).json().then(r => r.body),
                  $path: (option?: { method?: 'get' | undefined; query: Methods_1wx4fwn['get']['query'] } | undefined) =>
                    `${prefix}${prefix2}${PATH3}${option && option.query ? `?${dataToURLString(option.query)}` : ''}`,
                },
                /**
                 * get all data in repo
                 * @returns data
                 */
                get: (option?: { config?: T | undefined } | undefined) =>
                  fetch<Methods_rkwoeq['get']['resBody'], BasicHeaders, Methods_rkwoeq['get']['status']>(prefix, `${prefix2}${PATH2}`, GET, option).json(),
                /**
                 * get all data in repo
                 * @returns data
                 */
                $get: (option?: { config?: T | undefined } | undefined) =>
                  fetch<Methods_rkwoeq['get']['resBody'], BasicHeaders, Methods_rkwoeq['get']['status']>(prefix, `${prefix2}${PATH2}`, GET, option).json().then(r => r.body),
                /**
                 * create data
                 * @returns create data
                 */
                post: (option: { body: Methods_rkwoeq['post']['reqBody'], config?: T | undefined }) =>
                  fetch<Methods_rkwoeq['post']['resBody'], BasicHeaders, Methods_rkwoeq['post']['status']>(prefix, `${prefix2}${PATH2}`, POST, option).json(),
                /**
                 * create data
                 * @returns create data
                 */
                $post: (option: { body: Methods_rkwoeq['post']['reqBody'], config?: T | undefined }) =>
                  fetch<Methods_rkwoeq['post']['resBody'], BasicHeaders, Methods_rkwoeq['post']['status']>(prefix, `${prefix2}${PATH2}`, POST, option).json().then(r => r.body),
                $path: () => `${prefix}${prefix2}${PATH2}`,
              },
              properties: {
                _property_id: (val4: string) => {
                  const prefix4 = `${prefix2}${PATH4}/${val4}`;

                  return {
                    /**
                     * Delete property
                     * @returns Deleted
                     */
                    delete: (option?: { config?: T | undefined } | undefined) =>
                      fetch<Methods_lpkfsx['delete']['resBody'], BasicHeaders, Methods_lpkfsx['delete']['status']>(prefix, prefix4, DELETE, option).json(),
                    /**
                     * Delete property
                     * @returns Deleted
                     */
                    $delete: (option?: { config?: T | undefined } | undefined) =>
                      fetch<Methods_lpkfsx['delete']['resBody'], BasicHeaders, Methods_lpkfsx['delete']['status']>(prefix, prefix4, DELETE, option).json().then(r => r.body),
                    $path: () => `${prefix}${prefix4}`,
                  };
                },
                /**
                 * Get properties
                 * @returns OK
                 */
                get: (option?: { config?: T | undefined } | undefined) =>
                  fetch<Methods_xq9h73['get']['resBody'], BasicHeaders, Methods_xq9h73['get']['status']>(prefix, `${prefix2}${PATH4}`, GET, option).json(),
                /**
                 * Get properties
                 * @returns OK
                 */
                $get: (option?: { config?: T | undefined } | undefined) =>
                  fetch<Methods_xq9h73['get']['resBody'], BasicHeaders, Methods_xq9h73['get']['status']>(prefix, `${prefix2}${PATH4}`, GET, option).json().then(r => r.body),
                /**
                 * Add property
                 * @returns Created
                 */
                post: (option: { body: Methods_xq9h73['post']['reqBody'], config?: T | undefined }) =>
                  fetch<Methods_xq9h73['post']['resBody'], BasicHeaders, Methods_xq9h73['post']['status']>(prefix, `${prefix2}${PATH4}`, POST, option).json(),
                /**
                 * Add property
                 * @returns Created
                 */
                $post: (option: { body: Methods_xq9h73['post']['reqBody'], config?: T | undefined }) =>
                  fetch<Methods_xq9h73['post']['resBody'], BasicHeaders, Methods_xq9h73['post']['status']>(prefix, `${prefix2}${PATH4}`, POST, option).json().then(r => r.body),
                $path: () => `${prefix}${prefix2}${PATH4}`,
              },
              /**
               * Get a repo by id.
               * @returns A repo.
               */
              get: (option?: { config?: T | undefined } | undefined) =>
                fetch<Methods_chr4nt['get']['resBody'], BasicHeaders, Methods_chr4nt['get']['status']>(prefix, prefix2, GET, option).json(),
              /**
               * Get a repo by id.
               * @returns A repo.
               */
              $get: (option?: { config?: T | undefined } | undefined) =>
                fetch<Methods_chr4nt['get']['resBody'], BasicHeaders, Methods_chr4nt['get']['status']>(prefix, prefix2, GET, option).json().then(r => r.body),
              /**
               * Delete a repo by id.
               */
              delete: (option?: { config?: T | undefined } | undefined) =>
                fetch(prefix, prefix2, DELETE, option).send(),
              /**
               * Delete a repo by id.
               */
              $delete: (option?: { config?: T | undefined } | undefined) =>
                fetch(prefix, prefix2, DELETE, option).send().then(r => r.body),
              $path: () => `${prefix}${prefix2}`,
            };
          },
        };
      },
    },
    sign_up: {
      /**
       * Sign up
       * @returns Created
       */
      post: (option: { body: Methods_1uz1w0l['post']['reqBody'], config?: T | undefined }) =>
        fetch<Methods_1uz1w0l['post']['resBody'], BasicHeaders, Methods_1uz1w0l['post']['status']>(prefix, PATH5, POST, option).json(),
      /**
       * Sign up
       * @returns Created
       */
      $post: (option: { body: Methods_1uz1w0l['post']['reqBody'], config?: T | undefined }) =>
        fetch<Methods_1uz1w0l['post']['resBody'], BasicHeaders, Methods_1uz1w0l['post']['status']>(prefix, PATH5, POST, option).json().then(r => r.body),
      $path: () => `${prefix}${PATH5}`,
    },
  };
};

export type ApiInstance = ReturnType<typeof api>;
export default api;
