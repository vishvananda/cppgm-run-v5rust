template<class T>
T && move_ref(T & value)
{
  return static_cast<T&&>(value);
}

struct StringLike {
  template<class Operation>
  void resize_and_overwrite(int n, Operation op)
  {
    struct Terminator {
      ~Terminator() {}
      int result;
    };

    Terminator term = {0};
    int r = move_ref(op)((char*)0, (unsigned long)n);
    term.result = r;
  }
};

int helper(int value)
{
  StringLike s;
  s.resize_and_overwrite(1, [value](char *, unsigned long n) -> int {
    return value + (int)n;
  });
  return 0;
}

int main()
{
  return helper(1);
}
