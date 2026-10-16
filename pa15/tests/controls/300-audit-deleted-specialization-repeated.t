// expect exit 0
template<class T> int f(T) { return 1; }
template<> int f<int>(int) = delete;
template<> int f<int>(int) = delete;
template<> int f<int>(int);
int main() { return f('a') - 1; }
