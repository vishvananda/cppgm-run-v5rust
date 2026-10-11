// N3485 5.3.4/17 and 11.4/1 require access for the allocated object class.
struct Base {
protected:
  ~Base() {}
};
struct Derived : Base {
  static Base* create() { return new Base[2]; }
};
Base* use() { return Derived::create(); }
