struct C {};
template<int> struct leaf {};
leaf<([] { return 2 > 1; }())> value;
void parse_nested() {
  C object(int([] { typedef int local; local * value; return 1; }()));
  C([] { return [] { return 1; }(); }());
  typedef int word;
  auto first = [] { word * pointer; return 1; }(), word = 1;
  auto second = [] {
    struct local {
      local() : value([] { return sizeof(late); }()) {}
      int value;
      int method() { return [] { late * pointer; return 1; }(); }
      typedef int late;
    };
    return [] { return [] { return 1; }; };
  };
}
