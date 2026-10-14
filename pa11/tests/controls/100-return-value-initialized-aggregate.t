// N3485 8.5p7-8/6.6.3p2: return S(); value-initializes the returned trivial
// class object (zero-initializes y); expect exit 0
struct S { int y; };
static S make_s() { return S(); }
static int dirty() { volatile int a[8]; for (int i = 0; i < 8; ++i) a[i] = 1; return a[3]; }
int main() { dirty(); S s = make_s(); return s.y; }
