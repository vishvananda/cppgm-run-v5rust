int constructions;
int left_destructions;
int right_destructions;
int invalid_destructions;

struct Left
{
  int value;
  Left() : value(7) { ++constructions; }
  ~Left() noexcept
  {
    if (value != 7) ++invalid_destructions;
    ++left_destructions;
  }
};

struct Right
{
  int value;
  Right() : value(9) { ++constructions; }
  ~Right() noexcept
  {
    if (value != 9) ++invalid_destructions;
    ++right_destructions;
  }
};

int use(const Left &, bool fail)
{
  if (fail) throw 7;
  return 1;
}

int use(const Right &, bool fail)
{
  if (fail) throw 7;
  return 2;
}

void reset()
{
  constructions = left_destructions = right_destructions =
    invalid_destructions = 0;
}

int exercise(bool first, bool fail, bool same_type)
{
  reset();
  int result = 0;
  int caught = 0;
  try
  {
    if (same_type)
      result = first ? use(Left(), fail) : use(Left(), fail);
    else
      result = first ? use(Left(), fail) : use(Right(), fail);
  }
  catch (int error)
  {
    caught = error;
  }
  return constructions != 1 || invalid_destructions != 0 ||
    left_destructions != (first || same_type ? 1 : 0) ||
    right_destructions != (first || same_type ? 0 : 1) ||
    caught != (fail ? 7 : 0) ||
    result != (fail ? 0 : first || same_type ? 1 : 2);
}

int main()
{
  // Keep a local literal condition to exercise the former suppression path.
  bool first = false;
  reset();
  try
  {
    int result = first ? use(Left(), false) : use(Right(), true);
    (void)result;
  }
  catch (int)
  {
  }
  if (constructions != 1 || left_destructions != 0 ||
      right_destructions != 1 || invalid_destructions != 0) return 1;
  for (int side = 0; side != 2; ++side)
    for (int fail = 0; fail != 2; ++fail)
      for (int same = 0; same != 2; ++same)
        if (exercise(side, fail, same)) return 2;
  return 0;
}
