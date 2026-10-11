struct base { constexpr base() {} virtual int get() const = 0; };
struct object : base {
  constexpr object() : base() {}
  int get() const { return 1; }
};
object value;
