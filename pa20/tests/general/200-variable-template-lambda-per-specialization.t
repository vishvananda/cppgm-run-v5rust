// A lambda in a variable template's initializer is one closure type per
// specialization (14.7.1, 5.1.2p3), not one shared by whichever function
// first names a specialization.
template<class T> int (*width)() = [] { return int(sizeof(T)); };

namespace store
{
  template<class T> int (*tally)() = [] { static int calls = 0; return calls += int(sizeof(T)); };
}

int first() { return width<int>(); }
int second() { return width<char>(); }

int main()
{
  if (first() != 4 || second() != 1) return 10;
  if (store::tally<short>() != 2 || store::tally<char>() != 1) return 11;
  return store::tally<short>() == 4 ? 0 : 12;
}
