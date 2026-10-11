// A throw terminates one arm. Its cleanup region and temporary state must
// not become the starting state of the sibling or nested conditional.
int constructions;
int destructions;
int invalid_destructions;

struct Guard
{
  int value;
  Guard() : value(7) { ++constructions; }
  ~Guard()
  {
    if (value != 7) ++invalid_destructions;
    ++destructions;
  }
};

int use(const Guard &, bool fail)
{
  if (fail) throw 7;
  return 0;
}

int exercise(bool first, bool second)
{
  constructions = destructions = invalid_destructions = 0;
  int caught = 0;
  try
  {
    first ? (Guard(), throw 7) :
      (void)(second ? use(Guard(), true) : use(Guard(), false));
  }
  catch (int error)
  {
    caught = error;
  }
  return constructions != 1 || destructions != 1 ||
    invalid_destructions != 0 || caught != (first || second ? 7 : 0);
}

int main()
{
  for (int first = 0; first != 2; ++first)
    for (int second = 0; second != 2; ++second)
      if (exercise(first, second)) return 1;
  return 0;
}
