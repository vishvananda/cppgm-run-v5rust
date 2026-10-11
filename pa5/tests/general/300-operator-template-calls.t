template<class T> struct box { T value; };
template<class T> int operator+(box<T>, box<T>);
int compare(box<int> a, int y, int z) {
  return operator+<int>(a,a) < y > z;
}
template<char... Cs> unsigned long long operator "" _size();
auto size = operator "" _size<'1','2'>();
