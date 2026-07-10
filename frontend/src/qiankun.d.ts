/** qiankun 的 TypeScript 类型声明 */
declare module 'qiankun' {
  import type { RegistrableApp, ObjectType } from 'qiankun';

  export function registerMicroApps<T extends ObjectType>(
    apps: RegistrableApp<T>[],
    lifeCycles?: LifeCycles<T>,
  ): void;

  export function start(opts?: Options): void;

  export function loadMicroApp<T extends ObjectType>(
    app: LoadableApp<T>,
    configuration?: Configuration,
    lifeCycles?: LifeCycles<T>,
  ): MicroApp;

  export function initGlobalState(state: Record<string, unknown>): MicroAppStateActions;
}