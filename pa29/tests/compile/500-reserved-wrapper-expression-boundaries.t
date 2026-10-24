// Unsupported reserved calls defer only their owning full-expression.
// Typed locals, controlled statements and neighboring bodies keep their facts.
inline int conditions(int p) {
  int x = __builtin_unprovided_init(p);
  if (__builtin_unprovided_condition()) return x;
  if (int y = __builtin_unprovided_declaration()) return y;
  switch (__builtin_unprovided_switch()) { case 1: break; default: break; }
  while (__builtin_unprovided_while()) { ++p; break; }
  for (__builtin_unprovided_init_stmt(); __builtin_unprovided_test(); __builtin_unprovided_step()) { break; }
  do { ++p; } while (__builtin_unprovided_do());
  for (int item : __builtin_unprovided_range()) { p += item; }
  return p;
}
struct Wrapper {
  int value;
  Wrapper() : value(__builtin_unprovided_member()) {}
};
inline int fixed_lambda() {
  auto call = []() -> int { return __builtin_unprovided_lambda(); };
  return 0;
}
// A deferred initializer cannot deduce auto: its remaining body waits too.
inline int pending_auto() {
  auto value = __builtin_unprovided_auto();
  using Ignored = int;
  void ignored();
  if (int condition = 1) return condition;
  return deferred_body_name();
}
inline auto pending_result() { return __builtin_unprovided_result(); }
int ordinary_body() { int value = 3; return value + 1; }
// An explicit specialization owns this body; it is not an instantiated pattern.
template<class T> inline int specialized(T);
template<> inline int specialized<int>(int) { return __builtin_unprovided_specialization(); }
