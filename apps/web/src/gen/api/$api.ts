import type { AspidaClient, BasicHeaders } from 'aspida';
import { dataToURLString } from 'aspida';
import type { Methods as Methods_1voea85 } from './auth/v1beta/users';
import type { Methods as Methods_3vrui2 } from './auth/v1beta/verify';
import type { Methods as Methods_18qsrps } from './health';
import type { Methods as Methods_n80nxh } from './oauth/_provider_name@string/callback';
import type { Methods as Methods_nbtkog } from './oauth/_provider_name@string/connect';
import type { Methods as Methods_142eirx } from './v1beta/orgs';
import type { Methods as Methods_5mze20 } from './v1beta/orgs/_org@string';
import type { Methods as Methods_1hxkdmr } from './v1beta/repos';
import type { Methods as Methods_1idc6c2 } from './v1beta/repos/_org@string';
import type { Methods as Methods_318lkt } from './v1beta/repos/_org@string/_repo@string';
import type { Methods as Methods_17w3oi9 } from './v1beta/repos/_org@string/_repo@string/change-username';
import type { Methods as Methods_1vth81i } from './v1beta/repos/_org@string/_repo@string/data';
import type { Methods as Methods_1xjlc19 } from './v1beta/repos/_org@string/_repo@string/data/_data_id@string';
import type { Methods as Methods_1ip6svh } from './v1beta/repos/_org@string/_repo@string/data-list';
import type { Methods as Methods_p47per } from './v1beta/repos/_org@string/_repo@string/properties';
import type { Methods as Methods_1qm9cfp } from './v1beta/repos/_org@string/_repo@string/properties/_property_id@string';
import type { Methods as Methods_1refahk } from './v1beta/repos/_org@string/_repo@string/sources';
import type { Methods as Methods_1gn0zda } from './v1beta/repos/_org@string/_repo@string/sources/_source_id@string';

const api = <T>({ baseURL, fetch }: AspidaClient<T>) => {
  const prefix = (baseURL === undefined ? '' : baseURL).replace(/\/$/, '');
  const PATH0 = '/auth/v1beta/users';
  const PATH1 = '/auth/v1beta/verify';
  const PATH2 = '/health';
  const PATH3 = '/oauth';
  const PATH4 = '/callback';
  const PATH5 = '/connect';
  const PATH6 = '/v1beta/orgs';
  const PATH7 = '/v1beta/repos';
  const PATH8 = '/change-username';
  const PATH9 = '/data';
  const PATH10 = '/data-list';
  const PATH11 = '/properties';
  const PATH12 = '/sources';
  const GET = 'GET';
  const POST = 'POST';
  const PUT = 'PUT';
  const DELETE = 'DELETE';

  return {
    auth: {
      v1beta: {
        users: {
          /**
           * @returns User created successfully
           */
          post: (option: { body: Methods_1voea85['post']['reqBody'], config?: T | undefined }) =>
            fetch<Methods_1voea85['post']['resBody'], BasicHeaders, Methods_1voea85['post']['status']>(prefix, PATH0, POST, option).json(),
          /**
           * @returns User created successfully
           */
          $post: (option: { body: Methods_1voea85['post']['reqBody'], config?: T | undefined }) =>
            fetch<Methods_1voea85['post']['resBody'], BasicHeaders, Methods_1voea85['post']['status']>(prefix, PATH0, POST, option).json().then(r => r.body),
          $path: () => `${prefix}${PATH0}`,
        },
        verify: {
          /**
           * @returns Successful response
           */
          post: (option: { body: Methods_3vrui2['post']['reqBody'], config?: T | undefined }) =>
            fetch<Methods_3vrui2['post']['resBody'], BasicHeaders, Methods_3vrui2['post']['status']>(prefix, PATH1, POST, option).json(),
          /**
           * @returns Successful response
           */
          $post: (option: { body: Methods_3vrui2['post']['reqBody'], config?: T | undefined }) =>
            fetch<Methods_3vrui2['post']['resBody'], BasicHeaders, Methods_3vrui2['post']['status']>(prefix, PATH1, POST, option).json().then(r => r.body),
          $path: () => `${prefix}${PATH1}`,
        },
      },
    },
    health: {
      get: (option?: { config?: T | undefined } | undefined) =>
        fetch<void, BasicHeaders, Methods_18qsrps['get']['status']>(prefix, PATH2, GET, option).send(),
      $get: (option?: { config?: T | undefined } | undefined) =>
        fetch<void, BasicHeaders, Methods_18qsrps['get']['status']>(prefix, PATH2, GET, option).send().then(r => r.body),
      $path: () => `${prefix}${PATH2}`,
    },
    oauth: {
      _provider_name: (val1: string) => {
        const prefix1 = `${PATH3}/${val1}`;

        return {
          callback: {
            /**
             * @returns Successfully connected to provider
             */
            get: (option: { query: Methods_n80nxh['get']['query'], config?: T | undefined }) =>
              fetch<Methods_n80nxh['get']['resBody'], BasicHeaders, Methods_n80nxh['get']['status']>(prefix, `${prefix1}${PATH4}`, GET, option).json(),
            /**
             * @returns Successfully connected to provider
             */
            $get: (option: { query: Methods_n80nxh['get']['query'], config?: T | undefined }) =>
              fetch<Methods_n80nxh['get']['resBody'], BasicHeaders, Methods_n80nxh['get']['status']>(prefix, `${prefix1}${PATH4}`, GET, option).json().then(r => r.body),
            $path: (option?: { method?: 'get' | undefined; query: Methods_n80nxh['get']['query'] } | undefined) =>
              `${prefix}${prefix1}${PATH4}${option && option.query ? `?${dataToURLString(option.query)}` : ''}`,
          },
          connect: {
            /**
             * @returns Successfully generated authorization URL
             */
            get: (option?: { config?: T | undefined } | undefined) =>
              fetch<Methods_nbtkog['get']['resBody'], BasicHeaders, Methods_nbtkog['get']['status']>(prefix, `${prefix1}${PATH5}`, GET, option).json(),
            /**
             * @returns Successfully generated authorization URL
             */
            $get: (option?: { config?: T | undefined } | undefined) =>
              fetch<Methods_nbtkog['get']['resBody'], BasicHeaders, Methods_nbtkog['get']['status']>(prefix, `${prefix1}${PATH5}`, GET, option).json().then(r => r.body),
            $path: () => `${prefix}${prefix1}${PATH5}`,
          },
        };
      },
    },
    v1beta: {
      orgs: {
        _org: (val2: string) => {
          const prefix2 = `${PATH6}/${val2}`;

          return {
            /**
             * @returns Organization found
             */
            get: (option?: { config?: T | undefined } | undefined) =>
              fetch<Methods_5mze20['get']['resBody'], BasicHeaders, Methods_5mze20['get']['status']>(prefix, prefix2, GET, option).json(),
            /**
             * @returns Organization found
             */
            $get: (option?: { config?: T | undefined } | undefined) =>
              fetch<Methods_5mze20['get']['resBody'], BasicHeaders, Methods_5mze20['get']['status']>(prefix, prefix2, GET, option).json().then(r => r.body),
            /**
             * @returns Organization updated
             */
            put: (option: { body: Methods_5mze20['put']['reqBody'], config?: T | undefined }) =>
              fetch<Methods_5mze20['put']['resBody'], BasicHeaders, Methods_5mze20['put']['status']>(prefix, prefix2, PUT, option).json(),
            /**
             * @returns Organization updated
             */
            $put: (option: { body: Methods_5mze20['put']['reqBody'], config?: T | undefined }) =>
              fetch<Methods_5mze20['put']['resBody'], BasicHeaders, Methods_5mze20['put']['status']>(prefix, prefix2, PUT, option).json().then(r => r.body),
            $path: () => `${prefix}${prefix2}`,
          };
        },
        /**
         * @returns Organization created
         */
        post: (option: { body: Methods_142eirx['post']['reqBody'], config?: T | undefined }) =>
          fetch<Methods_142eirx['post']['resBody'], BasicHeaders, Methods_142eirx['post']['status']>(prefix, PATH6, POST, option).json(),
        /**
         * @returns Organization created
         */
        $post: (option: { body: Methods_142eirx['post']['reqBody'], config?: T | undefined }) =>
          fetch<Methods_142eirx['post']['resBody'], BasicHeaders, Methods_142eirx['post']['status']>(prefix, PATH6, POST, option).json().then(r => r.body),
        $path: () => `${prefix}${PATH6}`,
      },
      repos: {
        _org: (val2: string) => {
          const prefix2 = `${PATH7}/${val2}`;

          return {
            _repo: (val3: string) => {
              const prefix3 = `${prefix2}/${val3}`;

              return {
                change_username: {
                  /**
                   * @returns Repository username changed
                   */
                  put: (option: { body: Methods_17w3oi9['put']['reqBody'], config?: T | undefined }) =>
                    fetch<Methods_17w3oi9['put']['resBody'], BasicHeaders, Methods_17w3oi9['put']['status']>(prefix, `${prefix3}${PATH8}`, PUT, option).json(),
                  /**
                   * @returns Repository username changed
                   */
                  $put: (option: { body: Methods_17w3oi9['put']['reqBody'], config?: T | undefined }) =>
                    fetch<Methods_17w3oi9['put']['resBody'], BasicHeaders, Methods_17w3oi9['put']['status']>(prefix, `${prefix3}${PATH8}`, PUT, option).json().then(r => r.body),
                  $path: () => `${prefix}${prefix3}${PATH8}`,
                },
                data: {
                  _data_id: (val5: string) => {
                    const prefix5 = `${prefix3}${PATH9}/${val5}`;

                    return {
                      get: (option?: { config?: T | undefined } | undefined) =>
                        fetch(prefix, prefix5, GET, option).send(),
                      $get: (option?: { config?: T | undefined } | undefined) =>
                        fetch(prefix, prefix5, GET, option).send().then(r => r.body),
                      /**
                       * @returns Data updated
                       */
                      put: (option: { body: Methods_1xjlc19['put']['reqBody'], config?: T | undefined }) =>
                        fetch<Methods_1xjlc19['put']['resBody'], BasicHeaders, Methods_1xjlc19['put']['status']>(prefix, prefix5, PUT, option).json(),
                      /**
                       * @returns Data updated
                       */
                      $put: (option: { body: Methods_1xjlc19['put']['reqBody'], config?: T | undefined }) =>
                        fetch<Methods_1xjlc19['put']['resBody'], BasicHeaders, Methods_1xjlc19['put']['status']>(prefix, prefix5, PUT, option).json().then(r => r.body),
                      delete: (option?: { config?: T | undefined } | undefined) =>
                        fetch<void, BasicHeaders, Methods_1xjlc19['delete']['status']>(prefix, prefix5, DELETE, option).send(),
                      $delete: (option?: { config?: T | undefined } | undefined) =>
                        fetch<void, BasicHeaders, Methods_1xjlc19['delete']['status']>(prefix, prefix5, DELETE, option).send().then(r => r.body),
                      $path: () => `${prefix}${prefix5}`,
                    };
                  },
                  /**
                   * @returns Data found
                   */
                  get: (option: { query: Methods_1vth81i['get']['query'], config?: T | undefined }) =>
                    fetch<Methods_1vth81i['get']['resBody'], BasicHeaders, Methods_1vth81i['get']['status']>(prefix, `${prefix3}${PATH9}`, GET, option).json(),
                  /**
                   * @returns Data found
                   */
                  $get: (option: { query: Methods_1vth81i['get']['query'], config?: T | undefined }) =>
                    fetch<Methods_1vth81i['get']['resBody'], BasicHeaders, Methods_1vth81i['get']['status']>(prefix, `${prefix3}${PATH9}`, GET, option).json().then(r => r.body),
                  /**
                   * @returns Data created
                   */
                  post: (option: { body: Methods_1vth81i['post']['reqBody'], config?: T | undefined }) =>
                    fetch<Methods_1vth81i['post']['resBody'], BasicHeaders, Methods_1vth81i['post']['status']>(prefix, `${prefix3}${PATH9}`, POST, option).json(),
                  /**
                   * @returns Data created
                   */
                  $post: (option: { body: Methods_1vth81i['post']['reqBody'], config?: T | undefined }) =>
                    fetch<Methods_1vth81i['post']['resBody'], BasicHeaders, Methods_1vth81i['post']['status']>(prefix, `${prefix3}${PATH9}`, POST, option).json().then(r => r.body),
                  $path: (option?: { method?: 'get' | undefined; query: Methods_1vth81i['get']['query'] } | undefined) =>
                    `${prefix}${prefix3}${PATH9}${option && option.query ? `?${dataToURLString(option.query)}` : ''}`,
                },
                data_list: {
                  /**
                   * @returns Data list found
                   */
                  get: (option?: { config?: T | undefined } | undefined) =>
                    fetch<Methods_1ip6svh['get']['resBody'], BasicHeaders, Methods_1ip6svh['get']['status']>(prefix, `${prefix3}${PATH10}`, GET, option).json(),
                  /**
                   * @returns Data list found
                   */
                  $get: (option?: { config?: T | undefined } | undefined) =>
                    fetch<Methods_1ip6svh['get']['resBody'], BasicHeaders, Methods_1ip6svh['get']['status']>(prefix, `${prefix3}${PATH10}`, GET, option).json().then(r => r.body),
                  $path: () => `${prefix}${prefix3}${PATH10}`,
                },
                properties: {
                  _property_id: (val5: string) => {
                    const prefix5 = `${prefix3}${PATH11}/${val5}`;

                    return {
                      /**
                       * @returns Property found
                       */
                      get: (option?: { config?: T | undefined } | undefined) =>
                        fetch<Methods_1qm9cfp['get']['resBody'], BasicHeaders, Methods_1qm9cfp['get']['status']>(prefix, prefix5, GET, option).json(),
                      /**
                       * @returns Property found
                       */
                      $get: (option?: { config?: T | undefined } | undefined) =>
                        fetch<Methods_1qm9cfp['get']['resBody'], BasicHeaders, Methods_1qm9cfp['get']['status']>(prefix, prefix5, GET, option).json().then(r => r.body),
                      /**
                       * @returns Property updated
                       */
                      put: (option: { body: Methods_1qm9cfp['put']['reqBody'], config?: T | undefined }) =>
                        fetch<Methods_1qm9cfp['put']['resBody'], BasicHeaders, Methods_1qm9cfp['put']['status']>(prefix, prefix5, PUT, option).json(),
                      /**
                       * @returns Property updated
                       */
                      $put: (option: { body: Methods_1qm9cfp['put']['reqBody'], config?: T | undefined }) =>
                        fetch<Methods_1qm9cfp['put']['resBody'], BasicHeaders, Methods_1qm9cfp['put']['status']>(prefix, prefix5, PUT, option).json().then(r => r.body),
                      delete: (option?: { config?: T | undefined } | undefined) =>
                        fetch<void, BasicHeaders, Methods_1qm9cfp['delete']['status']>(prefix, prefix5, DELETE, option).send(),
                      $delete: (option?: { config?: T | undefined } | undefined) =>
                        fetch<void, BasicHeaders, Methods_1qm9cfp['delete']['status']>(prefix, prefix5, DELETE, option).send().then(r => r.body),
                      $path: () => `${prefix}${prefix5}`,
                    };
                  },
                  /**
                   * @returns Properties found
                   */
                  get: (option?: { config?: T | undefined } | undefined) =>
                    fetch<Methods_p47per['get']['resBody'], BasicHeaders, Methods_p47per['get']['status']>(prefix, `${prefix3}${PATH11}`, GET, option).json(),
                  /**
                   * @returns Properties found
                   */
                  $get: (option?: { config?: T | undefined } | undefined) =>
                    fetch<Methods_p47per['get']['resBody'], BasicHeaders, Methods_p47per['get']['status']>(prefix, `${prefix3}${PATH11}`, GET, option).json().then(r => r.body),
                  /**
                   * @returns Property created
                   */
                  post: (option: { body: Methods_p47per['post']['reqBody'], config?: T | undefined }) =>
                    fetch<Methods_p47per['post']['resBody'], BasicHeaders, Methods_p47per['post']['status']>(prefix, `${prefix3}${PATH11}`, POST, option).json(),
                  /**
                   * @returns Property created
                   */
                  $post: (option: { body: Methods_p47per['post']['reqBody'], config?: T | undefined }) =>
                    fetch<Methods_p47per['post']['resBody'], BasicHeaders, Methods_p47per['post']['status']>(prefix, `${prefix3}${PATH11}`, POST, option).json().then(r => r.body),
                  $path: () => `${prefix}${prefix3}${PATH11}`,
                },
                sources: {
                  _source_id: (val5: string) => {
                    const prefix5 = `${prefix3}${PATH12}/${val5}`;

                    return {
                      /**
                       * @returns Source found
                       */
                      get: (option?: { config?: T | undefined } | undefined) =>
                        fetch<Methods_1gn0zda['get']['resBody'], BasicHeaders, Methods_1gn0zda['get']['status']>(prefix, prefix5, GET, option).json(),
                      /**
                       * @returns Source found
                       */
                      $get: (option?: { config?: T | undefined } | undefined) =>
                        fetch<Methods_1gn0zda['get']['resBody'], BasicHeaders, Methods_1gn0zda['get']['status']>(prefix, prefix5, GET, option).json().then(r => r.body),
                      /**
                       * @returns Source updated
                       */
                      put: (option: { body: Methods_1gn0zda['put']['reqBody'], config?: T | undefined }) =>
                        fetch<Methods_1gn0zda['put']['resBody'], BasicHeaders, Methods_1gn0zda['put']['status']>(prefix, prefix5, PUT, option).json(),
                      /**
                       * @returns Source updated
                       */
                      $put: (option: { body: Methods_1gn0zda['put']['reqBody'], config?: T | undefined }) =>
                        fetch<Methods_1gn0zda['put']['resBody'], BasicHeaders, Methods_1gn0zda['put']['status']>(prefix, prefix5, PUT, option).json().then(r => r.body),
                      delete: (option?: { config?: T | undefined } | undefined) =>
                        fetch<void, BasicHeaders, Methods_1gn0zda['delete']['status']>(prefix, prefix5, DELETE, option).send(),
                      $delete: (option?: { config?: T | undefined } | undefined) =>
                        fetch<void, BasicHeaders, Methods_1gn0zda['delete']['status']>(prefix, prefix5, DELETE, option).send().then(r => r.body),
                      $path: () => `${prefix}${prefix5}`,
                    };
                  },
                  /**
                   * @returns Sources found
                   */
                  get: (option?: { config?: T | undefined } | undefined) =>
                    fetch<Methods_1refahk['get']['resBody'], BasicHeaders, Methods_1refahk['get']['status']>(prefix, `${prefix3}${PATH12}`, GET, option).json(),
                  /**
                   * @returns Sources found
                   */
                  $get: (option?: { config?: T | undefined } | undefined) =>
                    fetch<Methods_1refahk['get']['resBody'], BasicHeaders, Methods_1refahk['get']['status']>(prefix, `${prefix3}${PATH12}`, GET, option).json().then(r => r.body),
                  /**
                   * @returns Source created
                   */
                  post: (option: { body: Methods_1refahk['post']['reqBody'], config?: T | undefined }) =>
                    fetch<Methods_1refahk['post']['resBody'], BasicHeaders, Methods_1refahk['post']['status']>(prefix, `${prefix3}${PATH12}`, POST, option).json(),
                  /**
                   * @returns Source created
                   */
                  $post: (option: { body: Methods_1refahk['post']['reqBody'], config?: T | undefined }) =>
                    fetch<Methods_1refahk['post']['resBody'], BasicHeaders, Methods_1refahk['post']['status']>(prefix, `${prefix3}${PATH12}`, POST, option).json().then(r => r.body),
                  $path: () => `${prefix}${prefix3}${PATH12}`,
                },
                /**
                 * @returns Repository found
                 */
                get: (option?: { config?: T | undefined } | undefined) =>
                  fetch<Methods_318lkt['get']['resBody'], BasicHeaders, Methods_318lkt['get']['status']>(prefix, prefix3, GET, option).json(),
                /**
                 * @returns Repository found
                 */
                $get: (option?: { config?: T | undefined } | undefined) =>
                  fetch<Methods_318lkt['get']['resBody'], BasicHeaders, Methods_318lkt['get']['status']>(prefix, prefix3, GET, option).json().then(r => r.body),
                /**
                 * @returns Repository updated
                 */
                put: (option: { body: Methods_318lkt['put']['reqBody'], config?: T | undefined }) =>
                  fetch<Methods_318lkt['put']['resBody'], BasicHeaders, Methods_318lkt['put']['status']>(prefix, prefix3, PUT, option).json(),
                /**
                 * @returns Repository updated
                 */
                $put: (option: { body: Methods_318lkt['put']['reqBody'], config?: T | undefined }) =>
                  fetch<Methods_318lkt['put']['resBody'], BasicHeaders, Methods_318lkt['put']['status']>(prefix, prefix3, PUT, option).json().then(r => r.body),
                delete: (option?: { config?: T | undefined } | undefined) =>
                  fetch<void, BasicHeaders, Methods_318lkt['delete']['status']>(prefix, prefix3, DELETE, option).send(),
                $delete: (option?: { config?: T | undefined } | undefined) =>
                  fetch<void, BasicHeaders, Methods_318lkt['delete']['status']>(prefix, prefix3, DELETE, option).send().then(r => r.body),
                $path: () => `${prefix}${prefix3}`,
              };
            },
            /**
             * @returns Repository created
             */
            post: (option: { body: Methods_1idc6c2['post']['reqBody'], config?: T | undefined }) =>
              fetch<Methods_1idc6c2['post']['resBody'], BasicHeaders, Methods_1idc6c2['post']['status']>(prefix, prefix2, POST, option).json(),
            /**
             * @returns Repository created
             */
            $post: (option: { body: Methods_1idc6c2['post']['reqBody'], config?: T | undefined }) =>
              fetch<Methods_1idc6c2['post']['resBody'], BasicHeaders, Methods_1idc6c2['post']['status']>(prefix, prefix2, POST, option).json().then(r => r.body),
            $path: () => `${prefix}${prefix2}`,
          };
        },
        /**
         * @returns Repositories found
         */
        get: (option?: { config?: T | undefined } | undefined) =>
          fetch<Methods_1hxkdmr['get']['resBody'], BasicHeaders, Methods_1hxkdmr['get']['status']>(prefix, PATH7, GET, option).json(),
        /**
         * @returns Repositories found
         */
        $get: (option?: { config?: T | undefined } | undefined) =>
          fetch<Methods_1hxkdmr['get']['resBody'], BasicHeaders, Methods_1hxkdmr['get']['status']>(prefix, PATH7, GET, option).json().then(r => r.body),
        $path: () => `${prefix}${PATH7}`,
      },
    },
  };
};

export type ApiInstance = ReturnType<typeof api>;
export default api;
