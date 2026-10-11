class YA {
protected:
  int f() { return 4; }
};

class YB : public YA {
public:
  int g() { return YA::f(); }
};

int main() {
  YB p;
  return p.g();
}
