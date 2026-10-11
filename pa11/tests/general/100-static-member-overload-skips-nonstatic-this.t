struct C
{
  unsigned used() const { return 0; }
  static unsigned used(unsigned x) { return x; }
  static unsigned capacity(unsigned x) { return used(x); }
};

int main()
{
  return C::capacity(3) == 3 ? 0 : 1;
}
