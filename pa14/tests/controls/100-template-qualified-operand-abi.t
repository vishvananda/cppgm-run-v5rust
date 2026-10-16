// expect exit 0
// Qualified dependent calls preserve single-function and overload-set ABI forms.
namespace source {
int value = 2;
enum count { two = 2 };
long single(int x) { return x; }
int overloaded(int x) { return x; }
char overloaded(char x) { return x; }
}
struct provider {
  static int single(int x) { return x; }
  static int overloaded(int x) { return x; }
  static char overloaded(char x) { return x; }
};
int global_call(int x) { return x; }
char global_call(char x) { return x; }
template<class T> auto one(T x) -> decltype(source::single(x)) { return source::single(x); }
template<class T> auto many(T x) -> decltype(source::overloaded(x)) { return source::overloaded(x); }
template<class T> auto member_one(T x) -> decltype(provider::single(x)) { return provider::single(x); }
template<class T> auto member_many(T x) -> decltype(provider::overloaded(x)) { return provider::overloaded(x); }
template<class T> auto global(T x) -> decltype(::global_call(x)) { return ::global_call(x); }
template<class T> auto value(T x) -> decltype(source::value + x) { return source::value + x; }
template<class T> auto enumerator(T x) -> decltype(source::two + x) { return source::two + x; }
template<class T> struct shell;
template<class T> int nested(shell<decltype(source::single(T()))>*) { return 3; }
template<class T> int bound(T (&)[sizeof(source::single(T()))]) { return 3; }
int main() {
  int values[8];
  return one(3) == 3 && many(3) == 3 && member_one(3) == 3
      && member_many(3) == 3 && global(3) == 3 && value(3) == 5 && enumerator(3) == 5
      && nested<int>((shell<long>*)0) == 3 && bound(values) == 3 ? 0 : 1;
}
