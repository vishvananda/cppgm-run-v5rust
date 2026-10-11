class YB { public: virtual int f() { return 1; } virtual ~YB() {} };
class YD : public YB { public: int f() override { return 2; } };

int main() {
  YB b;
  YB *p = &b;
  if (dynamic_cast<YD*>(p)) return 0;
  return 1;
}
