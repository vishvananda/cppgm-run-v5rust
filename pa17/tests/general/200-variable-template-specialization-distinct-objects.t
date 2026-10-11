// Each specialization of a variable template is its own object with its
// own name (`_Z5widthIiE`, `_Z5widthIcE`, `_ZN5store5countIiEE`).
template<class T> int width = int(sizeof(T));
namespace store { template<class T> int count = 0; }

int* address_int() { return &width<int>; }
int* address_char() { return &width<char>; }

int main()
{
  if (address_int() == address_char()) return 10;
  store::count<int> = 3;
  store::count<char> = 4;
  if (store::count<int> != 3) return 11;
  return *address_int() == 4 && *address_char() == 1 ? 0 : 12;
}
