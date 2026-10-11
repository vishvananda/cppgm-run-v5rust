class YA { public: virtual int f() { return 1; } };
class YB { public: virtual int g() { return 2; } };
class YD : public YA, public YB { public: int g() override { return 4; } };
int call(YB &b) { return b.g(); }
int main() { YD d; return call(d); }
