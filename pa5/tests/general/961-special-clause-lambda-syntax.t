struct Defaults { Defaults(int = [] { int f(Unknown); return 0; }()); };
struct Exception { Exception() noexcept(noexcept([] { int f(Unknown); return 0; }())); };
struct Bound { Bound(int (*)[([] { int f(Unknown); return 1; }())]); };
template<class T> struct Pattern { Pattern(int = [] { int f(Unknown); return 0; }()); };
struct Conversion { operator int() noexcept(noexcept([] { int f(Unknown); return 0; }())); };
void ordinary(int = [] { int value(Unknown); return 0; }());
