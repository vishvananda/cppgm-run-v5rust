// N3485 14.5.3p7, 14.8.2.1p1: a bound pattern must have the call's length;
// expect reject
template<class T, class U> struct Pair {};
template<class... T> struct Owner
{
  template<class... U> int call(Pair<T, U>...) { return 0; }
};
int main() { Owner<int, long> o; return o.call(Pair<int, char>()); }
