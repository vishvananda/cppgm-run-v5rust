// expect reject
// Frozen substitute_template_name consumes the first qualified binding even
// when the lookup result also records ambiguity, retaining the class result.
struct First { template<class T> struct item {}; };
struct Second { template<class T> struct item {}; };
struct Qualifier : First, Second {};
template<class T> typename T::template item<int> select(T);
int select(...);
int main() {
  Qualifier value;
  return select(value) - 1;
}
