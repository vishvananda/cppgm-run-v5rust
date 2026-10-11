// A static data member template may be defined outside its class (14.5p1);
// the definition's initializer is the member's, analyzed in the class's
// scope (9.4.2p2), for an ordinary class and for a class template.
struct sizes
{
  static const int base = 2;
  template<class T> static const int padded;
  template<class T> static int counter;
};

template<class T> const int sizes::padded = int(sizeof(T)) + base;
template<class T> int sizes::counter = 7;

template<class U> struct pair_size
{
  template<class T> static const int total;
};

template<class U> template<class T>
const int pair_size<U>::total = int(sizeof(T) + sizeof(U));

int main()
{
  if (sizes::padded<short> != 4) return 10;
  sizes::counter<int> = sizes::counter<int> + 1;
  if (sizes::counter<int> != 8 || sizes::counter<char> != 7) return 11;
  return pair_size<char>::total<int> == 5 ? 0 : 12;
}
