// expect exit 0
// Array formation validates substituted elements, including unknown inner bounds.
using unknown = int[];
using known = int[2];
template<class T> char fixed(T (*)[3]);
template<class T> long fixed(...);
template<class T> char dependent(T (*)[sizeof(T*)]);
template<class T> long dependent(...);
int main() {
  if (sizeof(fixed<unknown>(0)) != sizeof(long)) return 1;
  if (sizeof(dependent<unknown>(0)) != sizeof(long)) return 2;
  if (sizeof(fixed<known>(0)) != sizeof(char)) return 3;
  if (sizeof(dependent<known>(0)) != sizeof(char)) return 4;
  if (sizeof(fixed<void>(0)) != sizeof(long)) return 5;
  return 0;
}
