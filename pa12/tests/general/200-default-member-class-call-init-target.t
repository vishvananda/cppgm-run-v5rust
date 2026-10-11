struct Iter {
  int value;
};

struct Range {
  Iter begin() const
  {
    Iter it = {1};
    return it;
  }
};

struct Holder {
  Range range;
  Iter iter = range.begin();
};

int main()
{
  Holder holder;
  return 0;
}
