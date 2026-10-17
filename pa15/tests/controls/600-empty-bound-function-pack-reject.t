// N3485 14.8.2p8: failed lockstep signature formation leaves no viable candidate.
// expect reject
template<class T, class U> struct Pair {};
template<class... T> struct Owner
{
  template<class... U> int call(Pair<T, U>...) { return 0; }
};
int main() { Owner<int> one; return one.call(); }
