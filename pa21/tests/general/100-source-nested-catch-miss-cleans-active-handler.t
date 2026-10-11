int destroyed = 0;

struct Guard
{
  ~Guard()
  {
    destroyed = destroyed + 1;
  }
};

void throw_long()
{
  throw 7L;
}

int main()
{
  try {
    try {
      throw 1;
    } catch(...) {
      Guard guard;
      try {
        throw_long();
      } catch(int) {
        return 2;
      }
    }
  } catch(long) {
    return destroyed == 1 ? 0 : 3;
  }
  return 4;
}
