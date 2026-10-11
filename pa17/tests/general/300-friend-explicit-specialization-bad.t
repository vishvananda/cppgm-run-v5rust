// N3485 14.7.3p18 (C++11 numbering 14.7.3p20 area): an explicit specialization
// declaration shall not be a friend declaration; expect reject
template<class T> int f(T) { return 0; }
struct S { template<> friend int f<int>(int); };
int main() { return 0; }
