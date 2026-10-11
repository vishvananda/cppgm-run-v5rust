// VALIDATION: compile-pass

struct Digest {
  unsigned char data[4];

  constexpr Digest() : data() {}

  constexpr int size() const
  {
    return 4;
  }
};

void test()
{
  constexpr Digest d;
  static_assert(d.size() == 4, "");
}

int main()
{
  test();
  return 0;
}
