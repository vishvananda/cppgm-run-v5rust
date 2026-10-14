// expect reject
struct Base { private: int select(int); };
struct Derived : Base {
  int select(int) { return 2; }
  using Base::select;
};
int main() { return 0; }
