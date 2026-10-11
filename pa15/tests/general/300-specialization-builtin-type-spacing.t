// HHC-028
template<class T> struct trait;
template<> struct trait<signed char> {};
template<> struct trait<unsigned long long> {};
int main() { return 0; }
