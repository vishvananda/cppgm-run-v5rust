template<class T>
struct X {
  template<class U>
  friend X<U> make(U);
};

int main() { return 0; }
