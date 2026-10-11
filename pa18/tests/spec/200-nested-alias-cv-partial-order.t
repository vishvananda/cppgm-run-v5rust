template<class T> struct item {};
struct source {
  template<class T> using value = item<T>;
  template<class T> using const_value = item<const T>;
};
template<class T> struct box {};
template<class T> int select(box<typename source::template value<T> >) { return 1; }
template<class T> char select(box<typename source::template const_value<T> >) { return 2; }
int main() { return select(box<item<const int> >()) == 2 ? 0 : 1; }
