// expect exit 0
struct Base { virtual ~Base() {} };
struct Derived : Base {};
struct Result { int n; };
void step() {}
Result query(Base& b) noexcept { step(); dynamic_cast<Derived&>(b); return Result{7}; }
int main() { Derived object; return query(object).n == 7 ? 0 : 1; }
