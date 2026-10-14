// N3485 6.5.4p1, 6.4p4: range-for's `__begin != __end` returning a class
// object is contextually converted to bool by the class's conversion function;
// expect exit 0
struct B { int v; operator bool() const { return v == 7; } };
struct It { int* p; B operator!=(It o) const { B b = {p != o.p ? 7 : 1}; return b; }
  int operator*() const { return *p; } It& operator++() { ++p; return *this; } };
struct R { int d[3]; It begin() { It i = {d}; return i; } It end() { It i = {d + 3}; return i; } };
int main() { R r = {{4, 5, 6}}; int s = 0; for (int x : r) s += x; return s == 15 ? 0 : 1; }
