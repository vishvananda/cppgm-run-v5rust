// expect exit 0
struct S { double x; const int *p; };
constexpr int value = 5;
constexpr S saved = {5.0, &value};
constexpr double S::*number = &S::x;
constexpr const int *S::*address = &S::p;
static_assert(bool(number) && bool{address}, "functional bool casts");
static_assert(saved.*number == 5.0, "floating member read");
static_assert(*(saved.*address) == 5, "pointer member read");
int main() { return 0; }
