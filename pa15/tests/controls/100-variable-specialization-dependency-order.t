// Already pending initializer dependencies retain depth-first emission; expect exit 0
struct Object { Object() = delete; };
template<class T> int value = 1;
template<> Object value<int>;
template<class T> Object* pointer = &value<int>;
template<class T> Object** indirect = &pointer<T>;
template<class T> int make_value() { return 2; }
template<class T> int dynamic_value = make_value<T>();
template<class T> int* dynamic_pointer = &dynamic_value<T>;
template<class T> int extra() { return 3; }
template<class T> extern void* first;
template<class T> void* second = &first<T>;
template<class T> void* first = &second<T>;
int main() {
  if (first<int> != &second<int> || second<int> != &first<int>) return 1;
  return (*indirect<char> != pointer<char>) + *dynamic_pointer<int>
      + extra<int>() + dynamic_value<int> - 7;
}
