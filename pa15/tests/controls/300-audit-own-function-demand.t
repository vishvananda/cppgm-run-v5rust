// expect exit 9
template<class T> inline int f(T x) { return sizeof(typename T::missing); }
template<> int f<int>(int declared);
template<> int f(int renamed) { return renamed + 1; }
int main() { return f(3) + f<int>(4); }
