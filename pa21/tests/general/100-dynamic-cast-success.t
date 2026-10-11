class YB { public: virtual int f() { return 1; } virtual ~YB() {} };
class YD : public YB { public: int f() override { return 2; } };

int main() {
  YD d;
  YB *b = &d;
  if (dynamic_cast<YD*>(b)) return 1;
  return 0;
}
