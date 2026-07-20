/* CSS Modules 全局类型声明。
 *
 * 所有 *.module.css import 默认导出一个 readonly 字符串字典
 * （className → 编译后的局部名）。
 */
declare module '*.module.css' {
  const classes: { readonly [key: string]: string };
  export default classes;
}
