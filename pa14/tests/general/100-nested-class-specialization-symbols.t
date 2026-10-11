template<typename T>
struct Box {
  struct Tag {
    Tag() {}
    ~Tag() {}
  };

  Tag tag;

  Box() {}
  ~Box() {}
};

long f() {
  Box<int> a;
  Box<char> b;
  return 0;
}
