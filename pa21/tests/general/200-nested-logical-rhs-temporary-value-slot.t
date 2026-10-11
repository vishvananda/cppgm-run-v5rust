// A nested logical expression evaluated as the temporary-owning RHS of an
// outer short-circuit condition still needs value-result storage.
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
  bool gate = true;
  if(gate && (use(Probe(), true) && true))
    ++destroyed;
  return 0;
}
