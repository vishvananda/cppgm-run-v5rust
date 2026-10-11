namespace std {
class type_info { public: bool operator==(const type_info&) const; bool operator!=(const type_info&) const; };
}

class YB { public: virtual int f() { return 1; } virtual ~YB() {} };
class YD : public YB { public: int f() override { return 2; } };

int main() {
  YD d;
  YB &b = d;
  if (typeid(b) == typeid(YD)) return 1;
  return 0;
}
