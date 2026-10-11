enum Source
{
  S0 = 0,
  S1 = 1
};

enum Target
{
  T0 = 0,
  T1 = 1
};

Target convert(Source source)
{
  return static_cast<Target>(source);
}
