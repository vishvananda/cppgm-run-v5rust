int gx;

struct YA {
  YA() { gx = gx + 1; }
  ~YA() { gx = gx + 2; }
};

struct YB {
  YA a;
};

int main() {
  {
    YB b;
  }
  return gx;
}
