// PA29 audit: designated list legality after target selection.
struct C { C(int); int x; };
template<class T> auto pick(int) -> decltype(T{.x=1}, int()) { return 1; }
template<class T> int pick(...) { return 2; }
int use() { return pick<C>(0); }
