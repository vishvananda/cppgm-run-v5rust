// EXIT_FAILURE
// A by-value complex parameter dies when its constexpr call returns.
typedef _Complex double Pair;
constexpr const Pair& borrowed(Pair value) { return value; }
constexpr double component = __real__ borrowed(__builtin_complex(1.0, 2.0));
