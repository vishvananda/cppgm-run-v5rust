namespace left {
int value;
}

namespace right {
int value;
}

template<class T>
int invalid_ambiguous_value()
{
  using namespace left;
  using namespace right;
  return value;
}
