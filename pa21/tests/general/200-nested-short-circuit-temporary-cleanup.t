// Nested logical lowering destroys only temporaries from evaluated arms,
// exactly once, before entering the selected statement arm.
int destroyed = 0;

struct Probe
{
  ~Probe() noexcept { ++destroyed; }
};

bool use(const Probe&, bool value) noexcept
{
  return value;
}

int main()
{
  if(use(Probe(), true) || (use(Probe(), true) && true))
  {
    if(destroyed != 1)
      return 1;
  }

  destroyed = 0;
  if(use(Probe(), false) || (use(Probe(), true) && true))
    return destroyed == 2 ? 0 : 2;
  return 3;
}
