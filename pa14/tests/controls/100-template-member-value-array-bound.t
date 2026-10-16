// expect exit 0
// Direct dependent values retain bound identity and instantiate in their own frame.
struct extent { static const int count = 3; };
struct wrapping { static const unsigned long count = 18446744073709551615UL; };
template<class T> int size(int (&values)[T::count]);
template<class U> int size(int (&values)[U::count]) { return sizeof(values); }
template<class T> int wrap(int (&values)[T::count + 4]) { return sizeof(values); }
template<class T, class U = int[T::count]> struct shell;
using instance = shell<extent>;
using instance = shell<extent, int[3]>;
template<class T> int unused(int (&)[T::unknown]);
int main() {
  int values[3];
  return size<extent>(values) == 12 && wrap<wrapping>(values) == 12 ? 0 : 1;
}
