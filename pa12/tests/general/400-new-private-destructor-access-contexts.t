// Scalar allocation does not require destructor access; array allocation does.
struct Hidden {
  int value;
  static Hidden* create_array() { return new Hidden[2]{}; }
private:
  ~Hidden() {}
};
decltype(new Hidden{7}) scalar();
Hidden* owned_array() { return Hidden::create_array(); }
