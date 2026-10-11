// N3485 focus: 10.2 [class.member.lookup] a derived declaration hides the same name in a base.
struct base { static const bool value = false; };
template<class> struct derived : base { static const bool value = true; };
typedef derived<int> alias;
typedef char check[alias::value ? 1 : -1];
int main() {}
