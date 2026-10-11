class YB { public: virtual int f() { return 1; } virtual ~YB() {} };
class YD : public YB { public: int f() override { return 2; } };
int main() { YD d; YB *b = &d; return dynamic_cast<void*>(b) != 0; }
