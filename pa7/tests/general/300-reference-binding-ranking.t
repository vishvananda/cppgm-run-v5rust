int f1() { return 0; }
int&& f2();
int g(const int&) { return 1; }
int g(const int&&) { return 2; }
int converted(const long long&) { return 5; }
int converted(const long long&&) { return 6; }
int sel(void(&)()) { return 3; }
int sel(void(&&)()) { return 4; }
void fn();
int h() { int i = 0; unsigned u = 0; return g(i) + g(f1()) + g(f2()) + sel(fn) + converted(u); }
