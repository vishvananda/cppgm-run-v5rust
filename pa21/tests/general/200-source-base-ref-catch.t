class B { public: virtual ~B() {} };
class D : public B {};

int main() {
  try {
    D d;
    throw d;
  } catch (B &b) {
    return 0;
  }
}
