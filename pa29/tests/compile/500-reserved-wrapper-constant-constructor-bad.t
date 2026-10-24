// The constructor keeps its mem-initializer even when typing is deferred.
struct Wrapper {
  int value;
  constexpr Wrapper() : value(__builtin_unprovided_constant()) {}
};
constexpr Wrapper object;
