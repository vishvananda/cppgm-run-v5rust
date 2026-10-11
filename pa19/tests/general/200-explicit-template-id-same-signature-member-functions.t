template<class T>
struct Box {
  template<class U>
  static int f() { return sizeof(U); }
};

int g() { return Box<int>::f<char>() + Box<int>::f<long>(); }
