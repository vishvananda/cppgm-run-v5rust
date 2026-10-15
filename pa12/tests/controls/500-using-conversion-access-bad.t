// expect reject
struct Base {
private:
  operator int() { return 7; }
};
struct Derived : Base { using Base::operator int; };
int main() {}
