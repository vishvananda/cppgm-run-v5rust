template<class T>
void accept(const T&);

struct Probe
{
  template<class T>
  static int test(const T&, decltype(accept<const T&>({}))* = 0)
  {
    return 1;
  }

  static char test(...);
};

int probe_size()
{
  return sizeof(Probe::test(0));
}

int invoke_probe()
{
  return Probe::test(0);
}
