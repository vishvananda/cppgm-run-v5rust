// VALIDATION: compile-fail
// N3485 focus: 10.3 [class.virtual]
// Expected: `override` requires a matching base virtual function.

class YB { public: int f() { return 1; } };
class YD : public YB { public: int f() override { return 2; } };
int main() { YD d; return d.f(); }
