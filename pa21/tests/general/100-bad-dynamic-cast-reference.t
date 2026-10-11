class YB { public: virtual ~YB() {} };
class YD : public YB {};

int main() {
  YD d;
  YB &b = d;
  dynamic_cast<YD&>(b);
  return 0;
}
