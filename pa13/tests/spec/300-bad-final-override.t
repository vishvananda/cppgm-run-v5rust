// VALIDATION: compile-fail
// N3485 focus: 10.3 [class.virtual]
// Expected: `final` rejects a later override of that virtual function.

class YB { public: virtual int f() { return 1; } virtual int g() { return 2; } };
class YD : public YB { public: int f() override { return 3; } virtual int g() final { return 4; } };
class YE : public YD { public: int g() override { return 5; } };
int main() { return 0; }
