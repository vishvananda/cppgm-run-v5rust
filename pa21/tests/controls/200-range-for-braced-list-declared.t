// N3485 8.5.4p2, 6.5.4p1: a braced-init-list range needs std::initializer_list
// declared before the loop; its declaration alone suffices, the list's array
// having the builtin initializer_list layout the loop iterates; expect exit 0
namespace std { template<class T> class initializer_list; }
struct P { int a, b; };
int main() {
  int s = 0;
  for (auto x : {1, 2, 3}) s += x;
  for (const auto& p : {P{1, 2}, P{3, 4}}) s += p.a * p.b;
  for (auto c : {'a', 'b'}) s += c - 'a';
  return s == 21 ? 0 : 1;
}
